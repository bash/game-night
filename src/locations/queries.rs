use super::LocationId;
use crate::auto_resolve;
use crate::infra::{DieselConnectionPool, DieselPoolConnection};
use crate::locations::{Location, Organizer, RawLocation, RawOrganizer};
use crate::users::User;
use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

auto_resolve! {
    pub(crate) struct LocationQueries {
          connection: DieselConnectionPool,
    }
}

impl LocationQueries {
    pub(crate) async fn all(&mut self) -> Result<Vec<Location>> {
        use crate::schema::locations::dsl::*;
        let mut connection = self.connection.get().await?;
        let all_locations = locations
            .select(RawLocation::as_select())
            .load(&mut connection)
            .await?;
        let organizers = fetch_organizers(&all_locations, &mut connection).await?;
        Ok(collect_locations(all_locations, organizers))
    }

    pub(crate) async fn by_id(&mut self, location_id: LocationId) -> Result<Option<Location>> {
        use crate::schema::locations::dsl::*;
        let mut connection = self.connection.get().await?;
        let Some(location) = locations
            .filter(id.eq(location_id))
            .select(RawLocation::as_select())
            .get_result(&mut connection)
            .await
            .optional()?
        else {
            return Ok(None);
        };
        // TODO: find a way to get rid of the clone here.
        let organizers =
            collect_organizers(fetch_organizers(&[location.clone()], &mut connection).await?);
        Ok(Some(Location {
            location,
            organizers,
        }))
    }
}

async fn fetch_organizers(
    locations: &[RawLocation],
    connection: &mut DieselPoolConnection,
) -> Result<Vec<(RawOrganizer, User)>> {
    use crate::schema::users;
    Ok(RawOrganizer::belonging_to(locations)
        .inner_join(users::table)
        .select((RawOrganizer::as_select(), User::as_select()))
        .load(connection)
        .await?)
}

fn collect_organizers(organizers: Vec<(RawOrganizer, User)>) -> Vec<Organizer> {
    organizers
        .into_iter()
        .map(|(organizer, user)| Organizer { organizer, user })
        .collect()
}

fn collect_locations(
    locations: Vec<RawLocation>,
    organizers: Vec<(RawOrganizer, User)>,
) -> Vec<Location> {
    organizers
        .grouped_by(&locations)
        .into_iter()
        .zip(locations)
        .map(|(organizers, location)| Location {
            location,
            organizers: collect_organizers(organizers),
        })
        .collect()
}
