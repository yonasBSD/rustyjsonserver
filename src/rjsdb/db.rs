use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::{rjsdb::{DbValue, FieldFilter, TableDb}};

#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum WalOp {
    CreateTable {
        table: String,
    },
    DropTable {
        table: String,
    },
    CreateEntry {
        table: String,
        id: String,
        value: DbValue,
    },
    UpdateEntry {
        table: String,
        id: String,
        value: DbValue,
    },
    DeleteEntry {
        table: String,
        id: String,
    },
}

#[derive(Serialize, Deserialize, Clone)]
struct Entry {
    value: DbValue,
}

#[derive(Serialize, Deserialize, Default)]
struct Snapshot {
    tables: HashMap<String, HashMap<String, Entry>>,
}

#[derive(Default)]
struct Inner {
    snap: Snapshot,
    wal: Option<File>,
}

pub struct JsonTableDb {
    dir: PathBuf,
    inner: Mutex<Inner>,
    id_counter: AtomicU64,
}

impl JsonTableDb {
    pub fn open<P: AsRef<Path>>(dir: P) -> io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let snap: Snapshot = Snapshot::default();
        let mut inner = Inner { snap, wal: None };

        let wal_path = dir.join("wal.jsonl");
        if wal_path.exists() {
            let f = File::open(&wal_path)?;
            for line in BufReader::new(f).lines() {
                if let Ok(line) = line {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(op) = json::from_str::<WalOp>(&line) {
                        apply_wal(&mut inner.snap, op);
                    }
                }
            }
        }

        inner.wal = Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&wal_path)?,
        );

        Ok(Self {
            dir,
            inner: Mutex::new(inner),
            id_counter: AtomicU64::new(seed_counter()),
        })
    }

    fn append(inner: &mut Inner, op: &WalOp) -> io::Result<()> {
        if let Some(wal) = &mut inner.wal {
            let line = serde_json::to_string(op)?;
            wal.write_all(line.as_bytes())?;
            wal.write_all(b"\n")?;
            wal.flush()?;
        }
        Ok(())
    }

    fn new_id(&self) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u128;
        let ctr = self.id_counter.fetch_add(1, Ordering::Relaxed) as u128;
        format!("{}-{}", base36_u128(nanos), base36_u128(ctr))
    }

    fn ensure_table<'a>(
        tables: &'a mut HashMap<String, HashMap<String, Entry>>,
        t: &str,
    ) -> &'a mut HashMap<String, Entry> {
        tables.entry(t.to_string()).or_default()
    }

    fn to_json(v: &DbValue) -> serde_json::Value {
        match v {
            DbValue::Number(n) => json::Value::from(*n),
            DbValue::Bool(b) => json::Value::from(*b),
            DbValue::String(s) => json::Value::from(s.clone()),
            DbValue::Null => json::Value::Null,
            DbValue::Json(j) => j.clone(),
        }
    }

    fn match_filter(val: &DbValue, filter: &FieldFilter) -> bool {
        if filter.is_empty() {
            return true;
        }
        match val {
            DbValue::Json(json::Value::Object(obj)) => {
                for (k, fv) in filter {
                    if let Some(v) = obj.get(k) {
                        if v != fv {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            _ => {
                if filter.len() != 1 {
                    return false;
                }
                if let Some(fv) = filter.get("$value") {
                    &Self::to_json(val) == fv
                } else {
                    false
                }
            }
        }
    }
}

fn apply_wal(snap: &mut Snapshot, op: WalOp) {
    match op {
        WalOp::CreateTable { table } => {
            snap.tables.entry(table).or_default();
        }
        WalOp::DropTable { table } => {
            snap.tables.remove(&table);
        }
        WalOp::CreateEntry { table, id, value } => {
            let t = snap.tables.entry(table).or_default();
            t.insert(id, Entry { value });
        }
        WalOp::UpdateEntry { table, id, value } => {
            if let Some(t) = snap.tables.get_mut(&table) {
                t.insert(id, Entry { value });
            }
        }
        WalOp::DeleteEntry { table, id } => {
            if let Some(t) = snap.tables.get_mut(&table) {
                t.remove(&id);
            }
        }
    }
}

fn seed_counter() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn base36_u128(mut x: u128) -> String {
    const ALPH: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if x == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while x > 0 {
        out.push(ALPH[(x % 36) as usize]);
        x /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

impl TableDb for JsonTableDb {
    fn create_table(&self, table: &str) -> io::Result<()> {
        let mut g = self.inner.lock().unwrap();
        g.snap.tables.entry(table.to_string()).or_default();
        JsonTableDb::append(
            &mut g,
            &WalOp::CreateTable {
                table: table.to_string(),
            },
        )
    }

    fn get_all_tables(&self) -> io::Result<Vec<String>> {
        let g = self.inner.lock().unwrap();
        Ok(g.snap.tables.keys().cloned().collect())
    }

    fn drop_table(&self, table: &str) -> io::Result<()> {
        let mut g = self.inner.lock().unwrap();
        g.snap.tables.remove(table);
        JsonTableDb::append(
            &mut g,
            &WalOp::DropTable {
                table: table.to_string(),
            },
        )
    }

    fn create_entry(&self, table: &str, value: DbValue) -> io::Result<String> {
        let mut g = self.inner.lock().unwrap();
        let id = self.new_id();
        let t = JsonTableDb::ensure_table(&mut g.snap.tables, table);

        t.insert(
            id.clone(),
            Entry {
                value: value.clone(),
            },
        );
        JsonTableDb::append(
            &mut g,
            &WalOp::CreateEntry {
                table: table.to_string(),
                id: id.clone(),
                value: value,
            },
        )?;
        Ok(id)
    }

    fn get_all(&self, table: &str) -> io::Result<Vec<(String, DbValue)>> {
        let g = self.inner.lock().unwrap();
        let mut out = Vec::new();
        if let Some(t) = g.snap.tables.get(table) {
            for (id, e) in t {
                out.push((id.clone(), e.value.clone()));
            }
        }
        Ok(out)
    }

    fn get_by_id(&self, table: &str, id: &str) -> io::Result<Option<(String, DbValue)>> {
        let g = self.inner.lock().unwrap();
        Ok(g.snap
            .tables
            .get(table)
            .and_then(|t| t.get(id))
            .map(|e| (id.to_string(), e.value.clone())))
    }

    fn get_by_fields(
        &self,
        table: &str,
        filter: &FieldFilter,
    ) -> io::Result<Vec<(String, DbValue)>> {
        let g = self.inner.lock().unwrap();
        let mut out = Vec::new();
        if let Some(t) = g.snap.tables.get(table) {
            for (id, e) in t {
                if JsonTableDb::match_filter(&e.value, filter) {
                    out.push((id.clone(), e.value.clone()));
                }
            }
        }
        Ok(out)
    }

    fn update_by_id(&self, table: &str, id: &str, patch: DbValue) -> io::Result<bool> {
        let mut g = self.inner.lock().unwrap();
        if let Some(t) = g.snap.tables.get_mut(table) {
            if let Some(ent) = t.get_mut(id) {
                ent.value = merge(ent.value.clone(), patch.clone());
                let new_value = ent.value.clone();
                JsonTableDb::append(
                    &mut g,
                    &WalOp::UpdateEntry {
                        table: table.to_string(),
                        id: id.to_string(),
                        value: new_value,
                    },
                )?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn update_by_fields(
        &self,
        table: &str,
        filter: &FieldFilter,
        patch: DbValue,
    ) -> io::Result<usize> {
        let mut g = self.inner.lock().unwrap();
        let mut updated = 0usize;
        let mut changes: Vec<(String, DbValue)> = Vec::new();

        if let Some(t) = g.snap.tables.get_mut(table) {
            let ids: Vec<String> = t
                .iter()
                .filter(|(_, e)| JsonTableDb::match_filter(&e.value, filter))
                .map(|(id, _)| id.clone())
                .collect();

            for id in ids {
                if let Some(ent) = t.get_mut(&id) {
                    ent.value = merge(ent.value.clone(), patch.clone());
                    changes.push((id, ent.value.clone()));
                    updated += 1;
                }
            }
        }

        for (id, val) in changes {
            JsonTableDb::append(
                &mut g,
                &WalOp::UpdateEntry {
                    table: table.to_string(),
                    id,
                    value: val,
                },
            )?;
        }

        Ok(updated)
    }

    fn delete_by_id(&self, table: &str, id: &str) -> io::Result<bool> {
        let mut g = self.inner.lock().unwrap();
        if let Some(t) = g.snap.tables.get_mut(table) {
            if t.remove(id).is_some() {
                JsonTableDb::append(
                    &mut g,
                    &WalOp::DeleteEntry {
                        table: table.to_string(),
                        id: id.to_string(),
                    },
                )?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn delete_by_fields(&self, table: &str, filter: &FieldFilter) -> io::Result<usize> {
        let mut g = self.inner.lock().unwrap();

        let ids: Vec<String> = if let Some(t) = g.snap.tables.get(table) {
            t.iter()
                .filter(|(_, e)| JsonTableDb::match_filter(&e.value, filter))
                .map(|(id, _)| id.clone())
                .collect()
        } else {
            return Ok(0);
        };

        let mut removed: Vec<String> = Vec::new();
        let mut deleted = 0usize;
        if let Some(t) = g.snap.tables.get_mut(table) {
            for id in &ids {
                if t.remove(id).is_some() {
                    removed.push(id.clone());
                    deleted += 1;
                }
            }
        }

        for id in removed {
            JsonTableDb::append(
                &mut g,
                &WalOp::DeleteEntry {
                    table: table.to_string(),
                    id,
                },
            )?;
        }

        Ok(deleted)
    }

    fn drop_db(&self) -> io::Result<()> {
        let mut g = self.inner.lock().unwrap();
        g.snap.tables.clear();
        let _ = fs::remove_file(self.dir.join("wal.jsonl"));
        // fresh WAL
        let wal_path = self.dir.join("wal.jsonl");
        g.wal = Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&wal_path)?,
        );
        Ok(())
    }
}

fn merge(orig: DbValue, patch: DbValue) -> DbValue {
    use serde_json::Value::Object;
    match (orig, patch) {
        (DbValue::Json(Object(mut base)), DbValue::Json(Object(p))) => {
            for (k, v) in p {
                base.insert(k, v);
            }
            DbValue::Json(json::Value::Object(base))
        }
        (_, p) => p,
    }
}
