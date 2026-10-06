// A virtual gamepad on /dev/uinput for pad_smoke.sh. Usage: vpad <name> <vendor> <product> <bus>, then lines on
// stdin: "k <key code> <0|1>", "a <abs code> <value>", "s <ms>" (sleep), "q" (unplug and exit).
#include <fcntl.h>
#include <linux/uinput.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <unistd.h>

static int fd;

static void emit(int type, int code, int value) {
    struct input_event ev;
    memset(&ev, 0, sizeof ev);
    ev.type = type;
    ev.code = code;
    ev.value = value;
    write(fd, &ev, sizeof ev);
}

static void absinfo(int code, int min, int max) {
    struct uinput_abs_setup s;
    memset(&s, 0, sizeof s);
    s.code = code;
    s.absinfo.minimum = min;
    s.absinfo.maximum = max;
    ioctl(fd, UI_SET_ABSBIT, code);
    ioctl(fd, UI_ABS_SETUP, &s);
}

int main(int argc, char **argv) {
    if (argc < 5) return 2;
    fd = open("/dev/uinput", O_WRONLY | O_NONBLOCK);
    if (fd < 0) { perror("open"); return 1; }
    ioctl(fd, UI_SET_EVBIT, EV_KEY);
    ioctl(fd, UI_SET_EVBIT, EV_ABS);
    int keys[] = {BTN_A, BTN_B, BTN_X, BTN_Y, BTN_TL, BTN_TR, BTN_SELECT, BTN_START, BTN_MODE, BTN_THUMBL, BTN_THUMBR};
    for (unsigned i = 0; i < sizeof keys / sizeof *keys; i++) ioctl(fd, UI_SET_KEYBIT, keys[i]);
    absinfo(ABS_HAT0X, -1, 1);
    absinfo(ABS_HAT0Y, -1, 1);
    absinfo(ABS_X, -32768, 32767);
    absinfo(ABS_Y, -32768, 32767);
    absinfo(ABS_RX, -32768, 32767);
    absinfo(ABS_RY, -32768, 32767);
    absinfo(ABS_Z, 0, 1023);
    absinfo(ABS_RZ, 0, 1023);
    struct uinput_setup us;
    memset(&us, 0, sizeof us);
    us.id.bustype = strtol(argv[4], 0, 0);
    us.id.vendor = strtol(argv[2], 0, 0);
    us.id.product = strtol(argv[3], 0, 0);
    us.id.version = 1;
    strncpy(us.name, argv[1], UINPUT_MAX_NAME_SIZE - 1);
    if (ioctl(fd, UI_DEV_SETUP, &us) < 0 || ioctl(fd, UI_DEV_CREATE) < 0) { perror("create"); return 1; }
    printf("created\n");
    fflush(stdout);
    char line[128];
    while (fgets(line, sizeof line, stdin)) {
        char c;
        int a = 0, b = 0;
        if (sscanf(line, " %c %i %i", &c, &a, &b) < 1) continue;
        if (c == 'q') break;
        if (c == 's') { usleep(a * 1000); continue; }
        emit(c == 'k' ? EV_KEY : EV_ABS, a, b);
        emit(EV_SYN, SYN_REPORT, 0);
    }
    ioctl(fd, UI_DEV_DESTROY);
    close(fd);
    return 0;
}
