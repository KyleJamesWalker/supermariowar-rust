// Prints struct sizes the port must match when it reads/writes raw C++ structs.
#include "GameModeSettings.h"
#include "input.h"
#include <cstdio>
#include <cstddef>

int main() {
    printf("GameModeSettings %zu\n", sizeof(GameModeSettings));
    printf("CInputPlayerControl %zu\n", sizeof(CInputPlayerControl));
#define OFF(f) printf("%s %zu\n", #f, offsetof(GameModeSettings, f));
    OFF(classic) OFF(frag) OFF(time) OFF(jail) OFF(coins) OFF(stomp) OFF(egg) OFF(flag) OFF(chicken) OFF(tag) OFF(star)
    OFF(domination) OFF(kingofthehill) OFF(race) OFF(frenzy) OFF(survival) OFF(greed) OFF(health) OFF(collection) OFF(chase) OFF(shyguytag) OFF(boss)
}
