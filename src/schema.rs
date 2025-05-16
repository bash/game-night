diesel::table! {
    users (id) {
        id -> BigInt,
        name -> Text,
        symbol -> Text,
        role -> Text,
        email_address -> Text,
        email_subscription -> Text,
        invited_by -> Nullable<BigInt>,
        campaign -> Nullable<Text>,
        can_update_name -> Bool,
        can_answer_strongly -> Bool,
        can_update_symbol -> Bool,
        last_active_at -> Text,
    }
}
