// @generated automatically by Diesel CLI.

diesel::table! {
    messages (id) {
        id -> Int4,
        user_id -> Int4,
        message -> Text,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        name -> Varchar,
        password -> Varchar,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    messages,
    users,
);
