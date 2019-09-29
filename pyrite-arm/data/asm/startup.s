.section INTERRUPT_VECTOR, "x"
.arm
.align 4
.global _start
.global _exit

_start:
    b reset_handler         @ reset
    b unhandled_exception   @ undefined
    b swi_handler           @ swi
    b reset_handler         @ prefetch abort
    b reset_handler         @ data abort
    b reset_handler         @ address exceeds 26bit
    b reset_handler         @ IRQ
    b reset_handler         @ FIQ

reset_handler:
    LDR sp, =stack_top
    bl  main
    b   _exit
    b   reset_handler

swi_handler:

unhandled_exception:
    @@TODO enable interrupts before doing the SWI
    swi 0xffffff        @ unhandled exception
    b   reset_handler

_exit:
    @@TODO enable interrupts before doing the SWI
    swi 17              @ bad halt
    b reset_handler
