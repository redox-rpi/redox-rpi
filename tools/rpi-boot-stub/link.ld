/* The Pi firmware and QEMU load kernel8.img at 0x80000 (text_offset in the
 * arm64 Image header). Everything the stub needs at run time lies between
 * __start and __end, page tables and stack included. The stub keeps its
 * zero-initialised statics in .data (src/main.rs), so kernel8.img contains
 * them and a loader that places the initramfs right after the file does
 * not overwrite them. */
ENTRY(_start)

SECTIONS {
    . = 0x80000;
    __start = .;
    .text : {
        KEEP(*(.text.head))
        *(.text*)
    }
    .rodata : ALIGN(4096) {
        *(.rodata.kernel)
        *(.rodata*)
    }
    .data : ALIGN(4096) {
        *(.data*)
        *(.bss*)
        *(COMMON)
        . = ALIGN(4096);
    }
    __end = .;
    __image_size = __end - __start;

    /DISCARD/ : { *(.comment*) *(.eh_frame*) *(.note*) }
}
