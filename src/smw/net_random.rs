//! Not in the C++ (PROGRESS.md, Netplay deviations): random outcomes that change a net game.
//!
//! Every client of a C++ net game rolls these from its own copy of the shared RNG, so the clients drift
//! apart as soon as one of them draws a different number of times. Each such site here is an `Ev`: the
//! site asks `event` first and runs its own code only when `event` returns false, which it does outside
//! gameplay, during a game's setup (while the clients still draw in step) and inside another event.
//! Otherwise `event` runs the site through `run` and writes its outcome to the harness dump as a `C`
//! record (REPLAY.md), so the clients' dumps can be compared.
//!
//! Objects made during setup and inside events get the same `iNetworkID` on every client, so events
//! can name them.

use crate::common::object_base::{set_network_id_context, CObjectTrait};
use crate::common::random_number_generator::RandomNumberGenerator;
use crate::globals::Ptr;
use crate::smw::gs_gameplay::{noncolcontainer, objectcontainer};
use crate::smw::harness;
use crate::smw::main::players;
use crate::smw::net::netplay;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Ev {
    FrenzyCard = 1,
    StompEnemy,
    SurvivalEnemy,
    CollectionCard,
    ReleaseCard,
    GreedCoins,
    Place,
    Wander,
    HazardTimer,
    Respawn,
    WarpExit,
    PodoboRain,
    Pow,
    MysterySwap,
    BulletBills,
    ShellDeath,
    Secret,
    ThrowBoxItem,
    StarTimeout,
    StarReassign,
    TagReassign,
    Bomb,
    Boomerang,
    PhantoReturn,
}

const EVENTS: [Ev; 24] = [
    Ev::FrenzyCard,
    Ev::StompEnemy,
    Ev::SurvivalEnemy,
    Ev::CollectionCard,
    Ev::ReleaseCard,
    Ev::GreedCoins,
    Ev::Place,
    Ev::Wander,
    Ev::HazardTimer,
    Ev::Respawn,
    Ev::WarpExit,
    Ev::PodoboRain,
    Ev::Pow,
    Ev::MysterySwap,
    Ev::BulletBills,
    Ev::ShellDeath,
    Ev::Secret,
    Ev::ThrowBoxItem,
    Ev::StarTimeout,
    Ev::StarReassign,
    Ev::TagReassign,
    Ev::Bomb,
    Ev::Boomerang,
    Ev::PhantoReturn,
];

impl Ev {
    pub fn from_u8(v: u8) -> Option<Ev> {
        EVENTS.iter().copied().find(|e| *e as u8 == v)
    }

    fn name(self) -> &'static str {
        match self {
            Ev::FrenzyCard => "frenzycard",
            Ev::StompEnemy => "stompenemy",
            Ev::SurvivalEnemy => "survivalenemy",
            Ev::CollectionCard => "collectioncard",
            Ev::ReleaseCard => "releasecard",
            Ev::GreedCoins => "greedcoins",
            Ev::Place => "place",
            Ev::Wander => "wander",
            Ev::HazardTimer => "hazardtimer",
            Ev::Respawn => "respawn",
            Ev::WarpExit => "warpexit",
            Ev::PodoboRain => "podoborain",
            Ev::Pow => "pow",
            Ev::MysterySwap => "mysteryswap",
            Ev::BulletBills => "bulletbills",
            Ev::ShellDeath => "shelldeath",
            Ev::Secret => "secret",
            Ev::ThrowBoxItem => "throwboxitem",
            Ev::StarTimeout => "startimeout",
            Ev::StarReassign => "starreassign",
            Ev::TagReassign => "tagreassign",
            Ev::Bomb => "bomb",
            Ev::Boomerang => "boomerang",
            Ev::PhantoReturn => "phantoreturn",
        }
    }
}

/// The objects added to the containers while an event runs, for its `C` record.
static mut g_added: Option<Vec<String>> = None;
static mut g_inSetup: bool = false;
static mut g_inGame: bool = false;

/// From the net game's sync until its first gameplay frame the clients draw in step and number objects alike.
pub fn begin_setup() {
    unsafe {
        g_inSetup = true;
        g_inGame = false;
        set_network_id_context(1);
    }
}

pub fn end_setup() {
    unsafe {
        if g_inSetup {
            g_inSetup = false;
            g_inGame = true;
            set_network_id_context(0);
        }
    }
}

/// Leaving gameplay: menus (map previews) roll for themselves again.
pub fn end_game() {
    unsafe {
        g_inSetup = false;
        g_inGame = false;
        set_network_id_context(0);
    }
}

/// True when the caller must not run the site's own code: this function already ran it.
pub fn event(ev: Ev, args: &[i32]) -> bool {
    unsafe {
        if !netplay.active || !g_inGame || g_added.is_some() || RandomNumberGenerator::tape_active() {
            return false;
        }
    }
    run_logged(ev, args);
    true
}

/// A placement that is not part of creating the object.
pub fn place_event(id: i32, extra: i32) -> bool {
    id != 0 && !find_object(id).is_null() && event(Ev::Place, &[id, extra])
}

/// An event about one object; objects that only one client numbered roll for themselves.
pub fn object_event(ev: Ev, args: &[i32]) -> bool {
    args[0] != 0 && event(ev, args)
}

pub(crate) fn run_logged(ev: Ev, args: &[i32]) {
    unsafe {
        g_added = Some(Vec::new());
        run(ev, args);
        let mut out = g_added.take().unwrap_or_default();
        out.extend(outcome(ev, args));
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        harness::note_net(format!("{} args={} out={}", ev.name(), args.join(","), out.join(";")));
    }
}

/// Called by `CObjectContainer::add_dyn`.
pub fn note_added(obj: Ptr<dyn CObjectTrait>) {
    unsafe {
        if let Some(added) = g_added.as_mut() {
            let mut o = obj;
            let moving = o.as_io_moving_object().map(|m| m.get_moving_object_type() as i32).unwrap_or(-1);
            added.push(format!("{}/{}@{},{}", o.get_object_type() as i32, moving, o.ix, o.iy));
        }
    }
}

pub fn note_powerup(iType: i16) {
    unsafe {
        if let Some(added) = g_added.as_mut() {
            added.push(format!("powerup{}", iType));
        }
    }
}

pub fn find_object(id: i32) -> Ptr<dyn CObjectTrait> {
    unsafe {
        if id == 0 {
            return Ptr::null();
        }
        for container in [&noncolcontainer, &objectcontainer[0], &objectcontainer[1], &objectcontainer[2]] {
            for &obj in container.list() {
                if obj.iNetworkID == id && !obj.dead {
                    return obj;
                }
            }
        }
        Ptr::null()
    }
}

pub fn wander_args(id: i32, fx: f32, fy: f32, angle: f32) -> [i32; 4] {
    [id, f32_arg(fx), f32_arg(fy), f32_arg(angle)]
}

pub fn awaiting_host() -> bool {
    false
}

pub fn f32_arg(v: f32) -> i32 {
    v.to_bits() as i32
}

pub fn arg_f32(v: i32) -> f32 {
    f32::from_bits(v as u32)
}

fn player(args: &[i32]) -> Ptr<crate::smw::player::CPlayer> {
    unsafe {
        let id = args[0];
        players.iter().copied().find(|p| p.globalID as i32 == id).unwrap_or(Ptr::null())
    }
}

fn run(ev: Ev, args: &[i32]) {
    use crate::smw::gamemodes as gm;
    use crate::smw::objects as obj;
    match ev {
        Ev::FrenzyCard => gm::frenzy::net_spawn_card(),
        Ev::StompEnemy => gm::stomp::net_spawn_enemy(),
        Ev::SurvivalEnemy => gm::survival::net_spawn_enemy(),
        Ev::CollectionCard => gm::card_collection::net_spawn_card(),
        Ev::ReleaseCard => gm::card_collection::net_release_card(player(args), args[1] as i16, args[2] as i16),
        Ev::GreedCoins => gm::greed::net_drop_coins(player(args), args[1] as i16, args[2] as i16, args[3] as i16),
        Ev::Place => obj::net_place(find_object(args[0]), args[1]),
        Ev::Wander => obj::net_wander(find_object(args[0]), args),
        Ev::HazardTimer => obj::net_hazard_timer(find_object(args[0])),
        Ev::Respawn => {
            let mut p = player(args);
            if !p.is_null() {
                p.net_respawn();
            }
        }
        Ev::WarpExit => {
            let mut p = player(args);
            if !p.is_null() {
                p.net_choose_warp_exit();
            }
        }
        Ev::PodoboRain => {
            let mut p = player(args);
            if !p.is_null() {
                p.net_podobo_rain();
            }
        }
        Ev::Pow => crate::smw::gs_gameplay::net_pow_kills(args),
        Ev::MysterySwap => {
            crate::smw::gs_gameplay::net_mystery_swap(args[0] as i16);
        }
        Ev::BulletBills => crate::smw::gs_gameplay::net_bullet_bill(args[0] as usize),
        Ev::ShellDeath => obj::carriable::co_shell::net_shell_death(find_object(args[0]), args[1]),
        Ev::Secret => crate::smw::objectgame::net_check_secret(args[0] as i16),
        Ev::ThrowBoxItem => obj::carriable::co_throw_box::net_release_item(args),
        Ev::StarTimeout => gm::star::net_timeout(),
        Ev::StarReassign => gm::star::net_reassign(),
        Ev::TagReassign => gm::tag::net_reassign(),
        Ev::Bomb => {
            let mut p = player(args);
            if !p.is_null() {
                p.net_throw_bomb(args[1], args[2], args[3] != 0);
            }
        }
        Ev::Boomerang => {
            let mut p = player(args);
            if !p.is_null() {
                p.net_throw_boomerang(args[1], args[2], args[3] != 0);
            }
        }
        Ev::PhantoReturn => obj::overmap::wo_phanto::net_return(find_object(args[0])),
    }
}

/// What each event changed besides adding objects.
fn outcome(ev: Ev, args: &[i32]) -> Vec<String> {
    match ev {
        Ev::Place | Ev::Wander | Ev::PhantoReturn => {
            let o = find_object(args[0]);
            if o.is_null() {
                vec!["gone".to_string()]
            } else {
                vec![format!("at{},{}", o.ix, o.iy)]
            }
        }
        Ev::Respawn | Ev::WarpExit => {
            let p = player(args);
            if p.is_null() {
                vec![]
            } else {
                vec![format!("p{}@{},{} s{}", p.globalID, p.ix, p.iy, p.state as i32)]
            }
        }
        Ev::HazardTimer => vec![crate::smw::objects::net_hazard_state(find_object(args[0]))],
        Ev::ShellDeath => {
            let o = find_object(args[0]);
            vec![if o.is_null() { "gone".to_string() } else { format!("dead{}", o.dead as i32) }]
        }
        Ev::StarTimeout | Ev::StarReassign => vec![gm_state_star()],
        Ev::TagReassign => vec![gm_state_tag()],
        _ => vec![],
    }
}

fn gm_state_star() -> String {
    crate::smw::gamemodes::star::net_state()
}

fn gm_state_tag() -> String {
    crate::smw::gamemodes::tag::net_state()
}
