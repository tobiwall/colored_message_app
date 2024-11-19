use diesel::result::DatabaseErrorKind;
use diesel::{prelude::*, Queryable, Insertable};
use diesel::pg::PgConnection;
use crystal_colors::schema::users;
use diesel::result::Error;
use crate::set_hash_password;

// Create a new user
#[derive(Queryable, Insertable, Debug)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub name: String,
    pub password: String,
    pub created_at: Option<chrono::NaiveDateTime>,
}
#[derive(Queryable, Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub name: String,
    pub password: String,
}

pub fn create_user(
    connection: &PgConnection,
    name: &str,
    password: &str,
) -> Result<User, diesel::result::Error> {

    let new_user = NewUser {
        name: name.to_string(),
        password: set_hash_password(password),
    };

    match diesel::insert_into(users::table)
    .values(&new_user)
    .get_result::<User>(connection)
{
    Ok(user) => Ok(user),
    Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, db_error_info)) => {
        println!("User with this name already exists.");
        Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, db_error_info))
    },
    Err(e) => Err(e),
}

}