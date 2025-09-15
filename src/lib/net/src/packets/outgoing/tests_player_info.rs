#[cfg(test)]
mod tests {
    use super::super::player_info_update::{PlayerWithActions, PlayerAction};
    use crate::skins_cache::{insert_skin, SkinProperties, clear_cache};

    #[test]
    fn test_add_and_remove_masks() {
        let add = PlayerWithActions::add_player(1234, "Alice");
        assert_eq!(add.get_actions_mask(), 0x01);

        let remove = PlayerWithActions::remove_player(1234);
        assert_eq!(remove.get_actions_mask(), 0x02);

        let both = PlayerWithActions { uuid: 1234, actions: vec![PlayerAction::AddPlayer { name: "A".into(), properties: ferrumc_net_codec::net_types::length_prefixed_vec::LengthPrefixedVec::default() }, PlayerAction::RemovePlayer {} ] };
        assert_eq!(both.get_actions_mask(), 0x03);
    }

    #[test]
    fn test_add_player_with_properties() {
        clear_cache();
        insert_skin(42, SkinProperties { name: "textures".to_string(), value: "dummyvalue".to_string(), signature: Some("sig".to_string()) });
        let p = PlayerWithActions::add_player_with_properties(42, "Bob");
        // mask should still be add
        assert_eq!(p.get_actions_mask(), 0x01);
    }
}
