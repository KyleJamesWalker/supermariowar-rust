// Constructs the real game modes for net_driver (GSMenu/main.cpp's create_gamemodes).
#include "GameValues.h"
#include "gamemodes/Classic.h"
#include "gamemodes/Frag.h"
#include "gamemodes/TimeLimit.h"
#include "gamemodes/Jail.h"
#include "gamemodes/Coins.h"
#include "gamemodes/Stomp.h"
#include "gamemodes/Eggs.h"
#include "gamemodes/CaptureTheFlag.h"
#include "gamemodes/Chicken.h"
#include "gamemodes/Tag.h"
#include "gamemodes/Star.h"
#include "gamemodes/Domination.h"
#include "gamemodes/KingOfTheHill.h"
#include "gamemodes/Race.h"
#include "gamemodes/Owned.h"
#include "gamemodes/Frenzy.h"
#include "gamemodes/Survival.h"
#include "gamemodes/Greed.h"
#include "gamemodes/Health.h"
#include "gamemodes/CardCollection.h"
#include "gamemodes/Chase.h"
#include "gamemodes/ShyGuyTag.h"
extern CGameMode* gamemodes[GAMEMODE_LAST];
extern short currentgamemode;
extern CGameValues game_values;
void create_gamemodes_for_driver() {
    gamemodes[0] = new CGM_Classic(); gamemodes[1] = new CGM_Frag(); gamemodes[2] = new CGM_TimeLimit(); gamemodes[3] = new CGM_Jail();
    gamemodes[4] = new CGM_Coins(); gamemodes[5] = new CGM_Stomp(); gamemodes[6] = new CGM_Eggs(); gamemodes[7] = new CGM_CaptureTheFlag();
    gamemodes[8] = new CGM_Chicken(); gamemodes[9] = new CGM_Tag(); gamemodes[10] = new CGM_Star(); gamemodes[11] = new CGM_Domination();
    gamemodes[12] = new CGM_KingOfTheHill(); gamemodes[13] = new CGM_Race(); gamemodes[14] = new CGM_Owned(); gamemodes[15] = new CGM_Frenzy();
    gamemodes[16] = new CGM_Survival(); gamemodes[17] = new CGM_Greed(); gamemodes[18] = new CGM_Health(); gamemodes[19] = new CGM_Collection();
    gamemodes[20] = new CGM_Chase(); gamemodes[21] = new CGM_ShyGuyTag();
    currentgamemode = 0;
    game_values.gamemode = gamemodes[currentgamemode];
}
