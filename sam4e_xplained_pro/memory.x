MEMORY
{
  FLASH (rx) : ORIGIN = 0x00400000, LENGTH = 1024K
  RAM (xrw)  : ORIGIN = 0x20000000, LENGTH = 128K
  CS0 (xrw)  : ORIGIN = 0x60000000, LENGTH = 16M
  CS1 (xrw)  : ORIGIN = 0x61000000, LENGTH = 16M
  CS2 (xrw)  : ORIGIN = 0x62000000, LENGTH = 16M
  CS3 (xrw)  : ORIGIN = 0x63000000, LENGTH = 16M
}
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
