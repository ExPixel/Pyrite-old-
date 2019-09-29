.text
.arm
.align 4

.global main

main:
    bl divide
    swi 16

@ inputs:
@   r0 - dividend
@   r1 - divisor
@ outputs:
@   r0 - result
@   r1 - remainder
divide:
    push    {r3, r4}

    mov r3, #1                  @ Bit to control the division
.div1:                          @ Move r1 until greater than r0
    cmp     r1, #0x80000000
    cmpcc   r1, r0
    movcc   r1, r1, asl #1
    movcc   r3, r3, asl #1
    bcc     .div1
    mov     r4, #0
.div2:
    cmp     r0, r1              @ Test for possible subtraction
    subcs   r0, r0, r1          @ Subtract if ok,
    addcs   r4, r4, r3          @ put relevant bit into result
    movs    r3, r3, lsr #1      @ shift control bit
    movne   r1, r1, lsr #1      @ halve unless finished
    bne     .div2
                                @ Divide result in r4
                                @ remainder in r0

    mov r1, r0                  @ move the remainder into r1
    mov r0, r4                  @ move the result into r0

    pop     {r3, r4}
    bx lr
