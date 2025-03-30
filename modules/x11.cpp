# include <node_api.h>
# include <X11/Xlib.h>
# include <X11/Xutil.h>
# include <X11/extensions/XTest.h>
# include <exception>


napi_value ClickMouse(napi_env env, napi_callback_info info) {
  try{

    Display* display = XOpenDisplay(NULL);
    if (display == NULL) {
        napi_value err = 0;
        return err;
    }

    size_t argc = 1;
    napi_value args[1];
    int button_;
    napi_get_value_int32(env, args[0], &button_);
    XTestFakeButtonEvent(display, button_, 1, 0);
    return 0;
  } catch (const std::exception& e) {

  }
}

napi_value Init(napi_env env, napi_value exports) {
    napi_value fn;
    napi_status status = napi_create_function(env, nullptr, 0, ClickMouse, nullptr, &fn);
    if (status != napi_ok) {
      napi_throw_error(env, nullptr, "Unable to wrap native function");
      return exports;
    }
  
    status = napi_set_named_property(env, exports, "ClickMouse", fn);
    if (status != napi_ok) {
      napi_throw_error(env, nullptr, "Unable to populate exports");
      return exports;
    }

    return exports;
}

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init)