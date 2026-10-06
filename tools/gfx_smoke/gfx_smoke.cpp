// C++ twin of crates/smw-core/examples/gfx_smoke.rs, linked against the original gfx sources.
#include "gfx.h"
#include "gfx/gfxFont.h"
#include "gfx/gfxSprite.h"

#include <cassert>
#include <cstdio>
#include <string>

SDL_Surface* screen = nullptr;
SDL_Surface* blitdest = nullptr;
short x_shake = 0;
short y_shake = 0;
std::string RootDataDirectory;

int main(int argc, char** argv)
{
    std::string data = argc > 1 ? argv[1] : "../../../data";
    std::string out = argc > 2 ? argv[2] : "gfx_smoke_cpp.bmp";
    RootDataDirectory = data;

    gfx_init(640, 480, false);
    blitdest = screen;

    std::string pack = data + "/gfx/packs/Classic";
    bool ok = gfx_loadpalette(pack + "/palette.png");
    assert(ok);

    gfxSprite backdrop = ImageLoader(pack + "/menu/menu_background.png").withoutColorKey().create();
    gfxSprite smw_logo = ImageLoader(pack + "/menu/menu_smw.png").create();
    gfxSprite shade = ImageLoader(pack + "/menu/menu_shade.png").withAlpha(72).withoutColorKey().create();
    gfxSprite ghost = ImageLoader(pack + "/eyecandy/ghost.png").withAlpha(128).withWrapping(640).create();
    gfxSprite overlay = ImageLoader(pack + "/eyecandy/overlayholes.png").withColorKey(RGB {0, 255, 0}).create();

    gfxFont font_large(pack + "/menu/menu_font_large.png");
    gfxFont font_small(pack + "/fonts/font_small.png");

    SpriteStrip skin = gfx_loadfullskin(data + "/gfx/skins/0smw.png", 1);
    SpriteStrip menuskin = gfx_loadmenuskin(data + "/gfx/skins/0smw.png", 2, true);

    backdrop.draw(0, 0);
    smw_logo.draw(70, 30);
    shade.draw(100, 200, {0, 0, 200, 100});
    ghost.draw(620, 300, {0, 0, 32, 32});
    overlay.draw(400, 350, {0, 0, 64, 64});
    for (int i = 0; i < 10; i++) {
        skin[i].draw(20 + i * 34, 400, {0, 0, 32, 32});
        skin[i].draw(20 + i * 34, 434, {32 * (i % 9), 0, 32, 32});
    }
    menuskin[0].draw(380, 400, {0, 0, 32, 32});
    menuskin[3].draw(414, 400, {96, 0, 32, 32});
    skin[0].draw(450, 400, {0, 0, 32, 32}, ClipEdge::Left, 460);
    skin[0].drawStretch({0, 0, 32, 32}, blitdest, {500, 380, 64, 64});
    gfx_drawpreview(skin[2], 620, 300, 0, 0, 32, 32, {0, 0, 640, 480}, true);

    font_large.draw(10, 150, "Super Mario War: 0123456789");
    font_small.drawCentered(320, 180, "centered small text ~!@#$%^&*()");
    font_large.drawRightJustified(630, 210, "right justified");
    font_small.drawChopRight(10, 250, 120, "chopped right text that is long");
    font_small.drawChopLeft(630, 250, 120, "chopped left text that is long");
    font_small.drawChopCentered(320, 270, 100, "chop centered text that is long");
    font_large.setAlpha(128);
    font_large.draw(10, 300, "translucent");
    x_shake = 3;
    y_shake = -2;
    font_small.draw(10, 330, "shaken");
    smw_logo.draw(300, 330, {0, 0, 40, 20});

    if (SDL_SaveBMP(screen, out.c_str()) != 0)
        return 1;
    printf("wrote %s\n", out.c_str());
    return 0;
}
