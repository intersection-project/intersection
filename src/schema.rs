// @generated automatically by Diesel CLI.

diesel::table! {
    guilds (id) {
        id -> Text,
        prefix -> Nullable<Text>,
    }
}
