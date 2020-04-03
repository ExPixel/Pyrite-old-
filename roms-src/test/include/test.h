#ifndef _test_h_
#define _test_h_

#include <gba_video.h>

#define TEST_STATUS *((volatile u32*)0x02000004)

typedef enum test_status {
    /*
     * Used at the top of every test to signal that the test is preparing
     * to be checked or executed.
     */
    test_status_setup = 0xDEADBEEF,

    /*
     * Used to signal the test driver that the setup process is done
     * and that the test can be checked or continued.
     */
    test_status_ready = 0xABCDEF01,

    /**
     * Used to return control back to the test driver.
     */
    test_status_break = 0xACFEBDBB,
} test_status;

#endif /* _test_h_ */
