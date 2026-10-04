// Linked with --pre-js. Emscripten's Browser.requestFullscreen sizes the canvas from
// canvas.widthNative/heightNative, which only Browser.setCanvasSize sets; SDL2 sizes the canvas
// itself, so they stay unset and entering fullscreen sets the canvas to 0x0 (upstream's web build
// has the same problem). Record the current size as the native one before going fullscreen.
Module['preRun'] = [].concat(Module['preRun'] || [], () => {
  const requestFullscreen = Module['requestFullscreen'];
  Module['requestFullscreen'] = (lockPointer, resizeCanvas) => {
    const canvas = Module['canvas'];
    if (!canvas.widthNative) {
      canvas.widthNative = canvas.width;
      canvas.heightNative = canvas.height;
    }
    return requestFullscreen(lockPointer, resizeCanvas);
  };
});
