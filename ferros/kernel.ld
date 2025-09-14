ENTRY(start)

SECTIONS {}

bss_start = ADDR(.bss);
bss_end = bss_start + SIZEOF(.bss);
