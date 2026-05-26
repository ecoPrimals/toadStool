/*
 * rm_trigger — Minimal RM ioctl client to trigger full GR initialization.
 *
 * Opens nvidiactl (minor 255), allocates root → device → subdevice,
 * which triggers RM's deferred GPU state loading (GPCCS/TPC init).
 *
 * Usage: rm_trigger <major>
 *   Creates /dev/toadstool-rm-ctl (major, minor 255) and issues ioctls.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <errno.h>
#include <stdint.h>

/* NVIDIA ioctl definitions (from nvidia-open-gpu-kernel-modules) */
#define NV_IOCTL_MAGIC      'F'
#define NV_ESC_CARD_INFO     0x23
#define NV_ESC_RM_ALLOC      0x2b
#define NV_ESC_RM_CONTROL    0x2a
#define NV_ESC_RM_FREE       0x29

/* NVOS64_PARAMETERS — used by NV_ESC_RM_ALLOC in 470.x */
typedef struct {
    uint32_t hRoot;
    uint32_t hObjectParent;
    uint32_t hObjectNew;
    uint32_t hClass;
    uint64_t pAllocParms __attribute__((aligned(8)));
    uint32_t paramsSize;
    uint32_t status;
} NVOS64_PARAMETERS;

/* NV0080_ALLOC_PARAMETERS — device allocation */
typedef struct {
    uint32_t deviceId;
    uint32_t hClientShare;
    uint32_t hTargetClient;
    uint32_t hTargetDevice;
    uint32_t flags;
    uint32_t _pad[3];
    uint64_t vaSpaceSize;
} NV0080_ALLOC_PARAMETERS;

/* NVOS54_PARAMETERS — used by NV_ESC_RM_CONTROL */
typedef struct {
    uint32_t hClient;
    uint32_t hObject;
    uint32_t cmd;
    uint32_t flags;
    uint64_t params __attribute__((aligned(8)));
    uint32_t paramsSize;
    uint32_t status;
} NVOS54_PARAMETERS;

#define RM_ALLOC_CMD  _IOWR(NV_IOCTL_MAGIC, NV_ESC_RM_ALLOC, NVOS64_PARAMETERS)
#define RM_CTRL_CMD   _IOWR(NV_IOCTL_MAGIC, NV_ESC_RM_CONTROL, NVOS54_PARAMETERS)

#define NV01_ROOT           0x0000
#define NV01_DEVICE_0       0x0080
#define NV20_SUBDEVICE_0    0x2080

static int rm_alloc(int fd, uint32_t root, uint32_t parent, uint32_t handle,
                    uint32_t cls, void *params, uint32_t params_size)
{
    NVOS64_PARAMETERS p = {0};
    p.hRoot = root;
    p.hObjectParent = parent;
    p.hObjectNew = handle;
    p.hClass = cls;
    p.pAllocParms = (uint64_t)(uintptr_t)params;
    p.paramsSize = params_size;
    p.status = 0xDEADBEEF;

    int rc = ioctl(fd, RM_ALLOC_CMD, &p);
    printf("  RM_ALLOC(cls=0x%04x): ioctl rc=%d errno=%d status=0x%x\n",
           cls, rc, rc < 0 ? errno : 0, p.status);
    if (rc < 0) return -errno;
    return (int)p.status;
}

int main(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <chardev_major>\n", argv[0]);
        return 1;
    }
    int major = atoi(argv[1]);

    printf("sizeof(NVOS64_PARAMETERS) = %zu\n", sizeof(NVOS64_PARAMETERS));
    printf("sizeof(NV0080_ALLOC_PARAMETERS) = %zu\n", sizeof(NV0080_ALLOC_PARAMETERS));
    printf("RM_ALLOC_CMD = 0x%lx\n", (unsigned long)RM_ALLOC_CMD);
    printf("RM_CTRL_CMD  = 0x%lx\n", (unsigned long)RM_CTRL_CMD);

    /* Create ctl device node (minor 255) */
    const char *ctl_path = "/dev/toadstool-rm-nvidiactl";
    unlink(ctl_path);
    if (mknod(ctl_path, S_IFCHR | 0666, makedev(major, 255)) < 0) {
        perror("mknod ctl");
        return 1;
    }

    /* Also create GPU device node (minor 0) for the open trigger */
    const char *gpu_path = "/dev/toadstool-rm-nvidia0";
    unlink(gpu_path);
    if (mknod(gpu_path, S_IFCHR | 0666, makedev(major, 0)) < 0) {
        perror("mknod gpu");
        unlink(ctl_path);
        return 1;
    }

    /* Open GPU device first — triggers rm_init_adapter */
    printf("\nOpening GPU device (minor 0) to trigger rm_init_adapter...\n");
    int gpu_fd = open(gpu_path, O_RDWR);
    if (gpu_fd < 0) {
        printf("  GPU open failed: %s (errno=%d)\n", strerror(errno), errno);
    } else {
        printf("  GPU open succeeded (fd=%d) — RM init triggered\n", gpu_fd);
    }

    /* Open ctl device for RM ioctls */
    printf("\nOpening nvidiactl (minor 255) for RM ioctls...\n");
    int ctl_fd = open(ctl_path, O_RDWR);
    if (ctl_fd < 0) {
        printf("  ctl open failed: %s (errno=%d)\n", strerror(errno), errno);
        goto cleanup;
    }
    printf("  ctl open succeeded (fd=%d)\n", ctl_fd);

    /* Step 1: Allocate root client */
    printf("\nStep 1: Allocating root client (NV01_ROOT)...\n");
    int s = rm_alloc(ctl_fd, 0, 0, 0xCAFE0001, NV01_ROOT, NULL, 0);
    if (s != 0) {
        printf("  Root alloc failed (status=0x%x)\n", s);
        goto cleanup;
    }

    /* Step 2: Allocate device */
    printf("\nStep 2: Allocating device (NV01_DEVICE_0)...\n");
    NV0080_ALLOC_PARAMETERS dev_params = {0};
    dev_params.deviceId = 0;
    s = rm_alloc(ctl_fd, 0xCAFE0001, 0xCAFE0001, 0xCAFE0002,
                 NV01_DEVICE_0, &dev_params, sizeof(dev_params));
    if (s != 0) {
        printf("  Device alloc failed (status=0x%x)\n", s);
        goto cleanup;
    }

    /* Step 3: Allocate subdevice */
    printf("\nStep 3: Allocating subdevice (NV20_SUBDEVICE_0)...\n");
    uint32_t sub_id = 0;
    s = rm_alloc(ctl_fd, 0xCAFE0001, 0xCAFE0002, 0xCAFE0003,
                 NV20_SUBDEVICE_0, &sub_id, sizeof(sub_id));
    if (s != 0) {
        printf("  Subdevice alloc failed (status=0x%x)\n", s);
        goto cleanup;
    }

    /* Step 4: Issue GR control to trigger deferred GR init */
    printf("\nStep 4: GR control (NV2080_CTRL_CMD_GR_GET_INFO)...\n");
    NVOS54_PARAMETERS ctrl = {0};
    ctrl.hClient = 0xCAFE0001;
    ctrl.hObject = 0xCAFE0003;
    ctrl.cmd = 0x20801201; /* NV2080_CTRL_CMD_GR_GET_INFO */
    ctrl.status = 0xDEADBEEF;
    int rc = ioctl(ctl_fd, RM_CTRL_CMD, &ctrl);
    printf("  GR_GET_INFO: ioctl rc=%d errno=%d status=0x%x\n",
           rc, rc < 0 ? errno : 0, ctrl.status);

    /* Hold fds open briefly for async RM work */
    printf("\nHolding fds open for 5s...\n");
    sleep(5);
    printf("Done.\n");

cleanup:
    if (ctl_fd >= 0) close(ctl_fd);
    if (gpu_fd >= 0) close(gpu_fd);
    unlink(ctl_path);
    unlink(gpu_path);
    return 0;
}
