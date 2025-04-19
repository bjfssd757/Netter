use diesel::prelude::*;
use crate::core::database::schema::users;
use crate::core::database::schema::users::dsl::*;

#[derive(Queryable, Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub password: String,
}

impl User {
    fn get_all(connection: &mut PgConnection) -> Vec<User> {
        users
            .load::<User>(connection)
            .expect("Error loading users")
    }
}