//! Bounded parent/child coordination SQL; services decide leases and owner approvals.
use crate::{Error, Repository};
use rusqlite::{OptionalExtension, params};
use zaturo_core::ComputerStep;
impl Repository<'_> {
    pub fn computer_step(&self, id: &str) -> Result<Option<ComputerStep>, Error> {
        let body: Option<String> = self
            .connection
            .query_row("SELECT body FROM computer_steps WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        body.map(|s| serde_json::from_str(&s).map_err(Error::from))
            .transpose()
    }
    pub fn computer_steps(&self, parent: &str) -> Result<Vec<ComputerStep>, Error> {
        let mut s = self.connection.prepare(
            "SELECT body FROM computer_steps WHERE parent_id=?1 ORDER BY sequence LIMIT 8",
        )?;
        s.query_map([parent], |r| r.get::<_, String>(0))?
            .map(|v| Ok(serde_json::from_str(&v?)?))
            .collect()
    }
    pub fn save_computer_step(&self, step: &ComputerStep) -> Result<(), Error> {
        self.connection.execute("INSERT INTO computer_steps(id,parent_id,sequence,body) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET body=excluded.body",params![step.id,step.parent_id,step.sequence,serde_json::to_string(step)?])?;
        Ok(())
    }
}
