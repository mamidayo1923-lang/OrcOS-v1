OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

PHDRS
{
    text    PT_LOAD    FLAGS((1 << 0) | (1 << 2)) ; /* 実行 + 読み取り */
    rodata  PT_LOAD    FLAGS((1 << 2)) ;            /* 読み取り専用 */
    data    PT_LOAD    FLAGS((1 << 1) | (1 << 2)) ; /* 書き込み + 読み取り */
    dynamic PT_DYNAMIC FLAGS((1 << 1) | (1 << 2)) ; /* 動的リンク (Limine用) */
}

SECTIONS
{
    /* Limineによって上位メモリにマッピングされるベースアドレス */
    . = 0xffffffff80000000;

    .text : {
        *(.text .text.*)
    } :text

    . += CONSTANT(MAXPAGESIZE);

    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    . += CONSTANT(MAXPAGESIZE);

    .data : {
        *(.data .data.*)
    } :data

    .dynamic : {
        *(.dynamic)
    } :data :dynamic

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}