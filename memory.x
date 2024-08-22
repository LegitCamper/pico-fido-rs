MEMORY {
    BOOT2   : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH   : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
    STORAGE : ORIGIN = ORIGIN(FLASH) + LENGTH(FLASH), LENGTH = 2048K
    RAM   : ORIGIN = 0x20000000, LENGTH = 264K
}
__flash_size = 4194304;
__storage_flash_size = 2097152;
__storage_flash_offset = ORIGIN(STORAGE) - ORIGIN(BOOT2);
