//! Port of src/smw/ui/MI_NetRoomTeamSelect.cpp

use crate::common::input::CPlayerInput;
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::{ui_control_modify, UI_Control, UI_ControlTrait};
use crate::globals::*;

pub struct MI_NetRoomTeamSelect {
    pub ui_control: UI_Control,

    x: i16,
    y: i16,
    owner_player: i16,

    onChangeAccepted: Box<dyn FnMut()>,
}
crate::impl_base!(MI_NetRoomTeamSelect => ui_control: UI_Control);

impl MI_NetRoomTeamSelect {
    pub fn new(x: i16, y: i16, player_id: i16, on_change_accepted: Box<dyn FnMut()>) -> Self {
        // TODO(mi_net_room_team_select): create miHoverImage / miModifyImage and animation state.
        MI_NetRoomTeamSelect { ui_control: UI_Control::new(x, y), x, y, owner_player: player_id, onChangeAccepted: on_change_accepted }
    }
}

impl UI_ControlTrait for MI_NetRoomTeamSelect {
    crate::impl_ctl!();

    fn update(&mut self) {
        // TODO(mi_net_room_team_select): port MI_NetRoomTeamSelect::Update.
    }

    fn draw(&mut self) {
        // TODO(mi_net_room_team_select): port MI_NetRoomTeamSelect::Draw.
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let _ = playerInput;
        // TODO(mi_net_room_team_select): port MI_NetRoomTeamSelect::SendInput.
        MENU_CODE_NONE
    }

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        // TODO(mi_net_room_team_select): port MI_NetRoomTeamSelect::Modify.
        ui_control_modify(self, modify)
    }
}
