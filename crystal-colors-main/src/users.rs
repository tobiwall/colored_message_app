use crate::password::hash_password;
use crate::schema::users;
use diesel::pg::PgConnection;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error;
use diesel::{prelude::*, Insertable, Queryable};

// A user in the DB
#[derive(Queryable, Insertable, Debug)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub name: String,
    pub password: String,
    pub created_at: Option<chrono::NaiveDateTime>,
}

// Create a new user in the DB
#[derive(Queryable, Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub name: String,
    pub password: String,
}

#[derive(Debug)]
pub enum UserError {
    UserAlreadyExists,
    DatabaseError(diesel::result::Error),
    AnyhowError(anyhow::Error),
}

impl From<diesel::result::Error> for UserError {
    fn from(e: diesel::result::Error) -> Self {
        match e {
            Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                UserError::UserAlreadyExists
            }
            _ => UserError::DatabaseError(e),
        }
    }
}

impl From<anyhow::Error> for UserError {
    fn from(e: anyhow::Error) -> Self {
        UserError::AnyhowError(e)
    }
}

pub fn create_user(
    connection: &PgConnection,
    name: &str,
    password: &str,
) -> Result<User, UserError> {
    let new_user = NewUser {
        name: name.to_string(),
        password: hash_password(password)?,
    };

    let user = diesel::insert_into(users::table)
        .values(&new_user)
        .get_result::<User>(connection)?;

    println!("Created user {:?}", user);
    Ok(user)
}
