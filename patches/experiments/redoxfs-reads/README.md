# Experiment: redoxfs-reads, which files a program reads

Test only ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)).
RedoxFS remembers, for each open file handle, the path, the bytes read and
the bytes mapped (`mmap`). When the handle is closed and it read or mapped
at least one block (4 KiB), RedoxFS prints

    redoxfs-reads: <path> read <bytes> mapped <bytes> in <n> maps

to its log (serial). Made for the COSMIC Text Editor, which keeps the SD
card driver busy while typing (Pi test #204). Files that stay open are not
printed until they are closed.
