// Prints sizeof/offsetof of every netplay package; the Rust protocol_packages test asserts the same numbers.
#include <cstddef>
#include <cstdio>
#include "input.h"
#include "GameModeSettings.h"
class CPlayer { public: float fx, fy, velx, vely, fOldY; short getGlobalID() const { return 0; } };
#include "network/ProtocolGamePackages.h"
using namespace NetPkgs;
#define S(T) printf(#T " %zu\n", sizeof(T));
#define O(T, f) printf(#T "." #f " %zu\n", offsetof(T, f));
union GMSU { ClassicGameModeSettings classic; FragGameModeSettings frag; TimeGameModeSettings time; JailGameModeSettings jail;
    CoinGameModeSettings coins; StompGameModeSettings stomp; EggGameModeSettings egg; FlagGameModeSettings flag;
    ChickenGameModeSettings chicken; TagGameModeSettings tag; StarGameModeSettings star; DominationGameModeSettings domination;
    KingOfTheHillModeSettings kingofthehill; RaceGameModeSettings race; FrenzyGameModeSettings frenzy; SurvivalGameModeSettings survival;
    GreedGameModeSettings greed; HealthGameModeSettings health; CollectionGameModeSettings collection; ChaseGameModeSettings chase;
    ShyGuyTagGameModeSettings shyguytag; GMSU() {} };
int main() {
    S(MessageHeader) S(ServerInfo) O(ServerInfo, name) O(ServerInfo, currentPlayerCount) O(ServerInfo, maxPlayerCount)
    S(ClientConnection) O(ClientConnection, playerName) S(ClientDisconnection) S(RoomList)
    S(RoomInfo) O(RoomInfo, roomID) O(RoomInfo, name) O(RoomInfo, currentPlayerCount) O(RoomInfo, passwordRequired) O(RoomInfo, gamemodeID)
    S(NewRoom) O(NewRoom, name) O(NewRoom, password) O(NewRoom, gamemodeID) O(NewRoom, gamemodeGoal)
    S(NewRoomCreated) O(NewRoomCreated, roomID) S(JoinRoom) O(JoinRoom, roomID) O(JoinRoom, password) S(LeaveRoom)
    S(CurrentRoom) O(CurrentRoom, roomID) O(CurrentRoom, name) O(CurrentRoom, playerName) O(CurrentRoom, hostPlayerNumber)
    O(CurrentRoom, remotePlayerNumber) O(CurrentRoom, gamemodeID) O(CurrentRoom, gamemodeGoal)
    S(RoomChatMsg) O(RoomChatMsg, senderNum) O(RoomChatMsg, message) S(StartRoom)
    S(GameHostInfo) O(GameHostInfo, host) S(PlayerInfo) O(PlayerInfo, host) O(PlayerInfo, port) S(StartSync) O(StartSync, commonRandomSeed)
    S(SyncOK) S(StartGame) S(LeaveGame) S(RawInput) S(ClientInput) O(ClientInput, input_id) O(ClientInput, input)
    S(RemoteInput) O(RemoteInput, playerNumber) O(RemoteInput, input)
    S(GameState) O(GameState, player_x) O(GameState, player_y) O(GameState, player_xvel) O(GameState, player_yvel) O(GameState, last_confirmed_local_input_id)
    S(RequestPowerup) S(StartPowerup) O(StartPowerup, player_id) O(StartPowerup, powerup_id) O(StartPowerup, delay)
    S(TriggerPowerup) O(TriggerPowerup, player_id) O(TriggerPowerup, powerup_id) O(TriggerPowerup, player_x) O(TriggerPowerup, player_y)
    S(MapCollision) O(MapCollision, player_id) O(MapCollision, player_x) O(MapCollision, player_y) O(MapCollision, player_xvel) O(MapCollision, player_yvel)
    S(P2PCollision) O(P2PCollision, player_id) O(P2PCollision, player_x) O(P2PCollision, player_y) O(P2PCollision, player_xvel) O(P2PCollision, player_yvel) O(P2PCollision, player_oldy)
    S(GMSU)
}
