ENTRY(_start)

SECTIONS
{
    . = 0x0;
    .text : {
        *startup.o (INTERRUPT_VECTOR)
        *(.text)
    }

    .data : { *(.data) }
    .bss : { *(.bss COMMON) }

    . = 0x1000; /* stack starts at 4KB*/
    stack_top = .;
}
