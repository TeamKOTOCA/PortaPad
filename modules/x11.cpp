# include <node_api.h>
# include <X11/Xlib.h>
# include <X11/Xutil.h>
# include <X11/extensions/XTest.h>


napi_value ClickMouse(napi_env env, napi_value button) {
    Display* display = XOpenDisplay(NULL);
    if (display == NULL) {
        return;
    }

    size_t argc = 1;
    napi_value args[1];
    int button_;
    napi_get_value_int32(env, args[0], &button_);
    XTestFakeButtonEvent(display, button_, 1, 0)
    return 0;
}