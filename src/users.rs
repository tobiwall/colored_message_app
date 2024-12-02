use crate::password::hash_password;
use crate::schema::users;
use diesel::pg::PgConnection;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error;
use diesel::{prelude::*, Insertable, Queryable};
use serde::Serialize;

// A user in the DB
#[derive(Queryable, Insertable, Debug, serde::Serialize)]
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

#[derive(Serialize, Debug)]
pub enum SignupResult {
    /// SignUp was successful. Contains the user id.
    Success(i32),
    /// SignUp was unsuccessful. Contains the error message.
    Failure(String),
}

pub fn create_user(connection: &mut PgConnection, name: &str, password: &str) -> SignupResult {
    let hashed_password = match hash_password(password) {
        Ok(hash) => hash,
        Err(_) => return SignupResult::Failure("Failed to hash password".to_string()),
    };

    let new_user = NewUser {
        name: name.to_string(),
        password: hashed_password,
    };

    match diesel::insert_into(users::table)
        .values(&new_user)
        .get_result::<User>(connection)
    {
        Ok(user) => {
            println!("Created user {:?}", user);
            SignupResult::Success(user.id)
        }
        Err(e) => {
            println!("Error creating user: {:?}", e);
            SignupResult::Failure(e.to_string())
        }
    }
}
