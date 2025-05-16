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

diesel::table! {
    invitations(id) {
        id -> BigInt,
        role -> Text,
        created_by -> Nullable<BigInt>,
        passphrase -> Text,
        comment -> Text,
        used_by -> Nullable<BigInt>,
        valid_until -> Nullable<Text>,
        created_at -> Text,
    }
}
