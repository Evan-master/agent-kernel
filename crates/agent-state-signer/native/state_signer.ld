ENTRY(state_signer_entry)

SECTIONS
{
    . = 0x0000400000000000;
    .text : ALIGN(16)
    {
        KEEP(*(.text.state_signer_entry))
        *(.text .text.*)
    }

    . = 0x0000400000010000;
    .rodata : ALIGN(16)
    {
        *(.rodata .rodata.*)
    }

    .data : { *(.data .data.*) }
    .bss : { *(.bss .bss.* COMMON) }

    ASSERT(SIZEOF(.data) == 0, "State Signer provider contains writable data")
    ASSERT(SIZEOF(.bss) == 0, "State Signer provider contains zero-initialized writable data")

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.note*)
    }
}
