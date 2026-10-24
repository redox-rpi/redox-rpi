//! Read or replace a file inside the RedoxFS partition of a Redox disk image.
//!
//! Usage:
//!   redoxfs-file <image> get <path-in-redoxfs> <local-file>
//!   redoxfs-file <image> put <path-in-redoxfs> <local-file>
//!
//! The RedoxFS partition is located through the GPT. For `put` the target
//! file must already exist; it is truncated and overwritten.

use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

use redoxfs::{Disk, FileSystem, TreePtr, BLOCK_SIZE};
use syscall::error::{Error, Result, EIO};

const SECTOR_SIZE: u64 = 512;

/// A partition of a disk image file, addressed in RedoxFS blocks.
struct PartitionDisk {
    file: File,
    offset: u64,
    size: u64,
}

impl Disk for PartitionDisk {
    unsafe fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> Result<usize> {
        self.file
            .read_at(buffer, self.offset + block * BLOCK_SIZE)
            .map_err(|_| Error::new(EIO))
    }

    unsafe fn write_at(&mut self, block: u64, buffer: &[u8]) -> Result<usize> {
        self.file
            .write_at(buffer, self.offset + block * BLOCK_SIZE)
            .map_err(|_| Error::new(EIO))
    }

    fn size(&mut self) -> Result<u64> {
        Ok(self.size)
    }
}

struct Partition {
    name: String,
    first_lba: u64,
    last_lba: u64,
}

fn read_u32(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(buf[at..at + 4].try_into().unwrap())
}

fn read_u64(buf: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(buf[at..at + 8].try_into().unwrap())
}

fn gpt_partitions(file: &File) -> std::io::Result<Vec<Partition>> {
    let mut header = [0; SECTOR_SIZE as usize];
    file.read_exact_at(&mut header, SECTOR_SIZE)?;
    if &header[0..8] != b"EFI PART" {
        return Err(std::io::Error::other("no GPT header found"));
    }

    let entries_lba = read_u64(&header, 72);
    let entry_count = read_u32(&header, 80) as usize;
    let entry_size = read_u32(&header, 84) as usize;
    let mut entries = vec![0; entry_count * entry_size];
    file.read_exact_at(&mut entries, entries_lba * SECTOR_SIZE)?;

    let mut partitions = Vec::new();
    for entry in entries.chunks_exact(entry_size) {
        // Unused entries have an all-zero type GUID.
        if entry[0..16].iter().all(|&b| b == 0) {
            continue;
        }
        let name: Vec<u16> = entry[56..128]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&c| c != 0)
            .collect();
        partitions.push(Partition {
            name: String::from_utf16_lossy(&name),
            first_lba: read_u64(entry, 32),
            last_lba: read_u64(entry, 40),
        });
    }
    Ok(partitions)
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("redoxfs-file: {msg}");
    exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 || !matches!(args[2].as_str(), "get" | "put") {
        eprintln!("usage: redoxfs-file <image> get|put <path-in-redoxfs> <local-file>");
        exit(1);
    }
    let (image, command, target, local) = (&args[1], &args[2], &args[3], &args[4]);

    let file = OpenOptions::new()
        .read(true)
        .write(command == "put")
        .open(image)
        .unwrap_or_else(|e| fail(format!("failed to open {image}: {e}")));

    let partitions =
        gpt_partitions(&file).unwrap_or_else(|e| fail(format!("failed to read partition table: {e}")));
    let Some(partition) = partitions.iter().find(|p| p.name.to_uppercase().contains("REDOX")) else {
        fail("no RedoxFS partition found");
    };
    let disk = PartitionDisk {
        file,
        offset: partition.first_lba * SECTOR_SIZE,
        size: (partition.last_lba + 1 - partition.first_lba) * SECTOR_SIZE,
    };

    // Only clean up (and thereby write) when we are going to modify the image.
    let mut fs = FileSystem::open(disk, None, None, command == "put")
        .unwrap_or_else(|e| fail(format!("failed to open RedoxFS on partition {:?}: {e}", partition.name)));

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let find = |tx: &mut redoxfs::Transaction<PartitionDisk>| -> Result<TreePtr<redoxfs::Node>> {
        let mut node_ptr = TreePtr::root();
        for name in target.split('/').filter(|s| !s.is_empty()) {
            node_ptr = tx.find_node(node_ptr, name)?.ptr();
        }
        Ok(node_ptr)
    };

    if command == "get" {
        let data = fs
            .tx(|tx| {
                let node_ptr = find(tx)?;
                let size = tx.read_tree(node_ptr)?.data().size();
                let mut data = vec![0; size as usize];
                // An atime of 0 never updates the node, so nothing is written.
                let read = tx.read_node(node_ptr, 0, &mut data, 0, 0)?;
                data.truncate(read);
                Ok(data)
            })
            .unwrap_or_else(|e| fail(format!("failed to read {target}: {e}")));
        std::fs::write(local, &data).unwrap_or_else(|e| fail(format!("failed to write {local}: {e}")));
        println!("read {} bytes from {target}", data.len());
    } else {
        let data = std::fs::read(local).unwrap_or_else(|e| fail(format!("failed to read {local}: {e}")));
        fs.tx(|tx| {
            let node_ptr = find(tx)?;
            tx.truncate_node(node_ptr, 0, now.as_secs(), now.subsec_nanos())?;
            let written = tx.write_node(node_ptr, 0, &data, now.as_secs(), now.subsec_nanos())?;
            assert_eq!(written, data.len(), "short write");
            Ok(())
        })
        .unwrap_or_else(|e| fail(format!("failed to write {target}: {e}")));
        println!("wrote {} bytes to {target} on partition {:?}", data.len(), partition.name);
    }
}
