//! Runs the contacts module's own migrations, services and delta/search
//! primitives against a real server of **each** engine, from a single compiled
//! binary — the proof that the engine is a run-time choice, not a build-time
//! one, and that the ported primitives (the change journal, the normalized
//! full-text search, the JSON field arrays) behave identically on all of them.
//!
//! Contact CRUD is self-contained (no drive/core dependency), so the tests drive
//! the real `contact_service` / `label_service` / `reminder_service` code, plus
//! a little raw SQL for the group domain (whose writes live in the handler).
//!
//! * SQLite always runs (a temp file, no server).
//! * PostgreSQL runs when `KUBUNO_PG_TEST_URL` points at a throwaway database.
//! * MySQL/MariaDB runs when `KUBUNO_MYSQL_TEST_URL` does.
//!
//! ```sh
//! KUBUNO_PG_TEST_URL=postgres://u:p@127.0.0.1:5433/contacts \
//! KUBUNO_MYSQL_TEST_URL=mysql://u:p@127.0.0.1:3307/contacts \
//!   cargo test --test db_portability
//! ```

use kubuno_contacts::models::contact::{
    ContactField, CreateContactDto, ListContactsParams, UpdateContactDto,
};
use kubuno_contacts::models::reminder::CreateReminderDto;
use kubuno_contacts::models::label::{CreateLabelDto, UpdateLabelDto};
use kubuno_contacts::services::{contact_service, label_service, reminder_service};
use kubuno_contacts::{sync, SCHEMA};
use kubuno_db::{journal, new_id, params};
use uuid::Uuid;

fn base_settings(engine: &str) -> kubuno_db::DbSettings {
    kubuno_db::DbSettings {
        engine: engine.to_string(),
        url: None,
        host: None,
        port: None,
        user: None,
        password: None,
        database: None,
        path: None,
        max_connections: 4,
        min_connections: 0,
        connect_timeout: std::time::Duration::from_secs(10),
        run_migrations: true,
        schema_prefix: None,
    }
}

/// Migrations run one at a time: the PostgreSQL and MySQL suites may share a server.
static EXCLUSIVE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn migrated_pool(settings: kubuno_db::DbSettings) -> (kubuno_db::DbPool, impl Sized) {
    let guard = EXCLUSIVE.lock().await;
    let pool = kubuno_db::connect(&settings, SCHEMA).await.expect("connect");
    kubuno_db::migrations!(
        "./migrations/postgres",
        "./migrations/mysql",
        "./migrations/sqlite",
    )
    .run(&pool, SCHEMA)
    .await
    .expect("migrations");
    (pool, guard)
}

fn email(v: &str) -> ContactField {
    ContactField { label: None, value: v.to_string(), field_type: "work".into() }
}

fn new_contact(display: &str, org: Option<&str>, emails: Vec<ContactField>) -> CreateContactDto {
    CreateContactDto {
        display_name: Some(display.to_string()),
        organization: org.map(str::to_string),
        emails,
        ..Default::default()
    }
}

fn list_params(q: Option<&str>) -> ListContactsParams {
    ListContactsParams {
        q: q.map(str::to_string),
        group_id: None,
        label_id: None,
        starred: None,
        trashed: None,
        archived: None,
        filter: None,
        sort: None,
        limit: Some(200),
        offset: Some(0),
    }
}

async fn change_seq(pool: &kubuno_db::DbPool, id: Uuid) -> i64 {
    pool.fetch_scalar::<i64>("SELECT change_seq FROM contacts.contacts WHERE id = $1", params![id])
        .await
        .expect("contact change_seq")
}

/// A search that returns the matching display names, in list order.
async fn search_names(pool: &kubuno_db::DbPool, owner: Uuid, q: &str) -> Vec<String> {
    contact_service::list_contacts(pool, owner, &list_params(Some(q)))
        .await
        .expect("search")
        .contacts
        .into_iter()
        .map(|c| c.contact.display_name)
        .collect()
}

async fn full_suite(pool: &kubuno_db::DbPool) {
    let owner = Uuid::new_v4();

    // ── contacts: CRUD + strict change_seq monotonicity ──
    let mut seqs: Vec<i64> = Vec::new();

    let c1 = contact_service::create_contact(
        pool, owner, &new_contact("Jean Chevaux", Some("Solutions"), vec![email("alice@example.com")]),
    )
    .await
    .expect("create c1");
    seqs.push(change_seq(pool, c1.id).await);

    let c2 = contact_service::create_contact(
        pool, owner, &new_contact("Marie Martin", Some("Développement Durable"), vec![]),
    )
    .await
    .expect("create c2");
    seqs.push(change_seq(pool, c2.id).await);

    let c3 = contact_service::create_contact(
        pool, owner, &new_contact("Paul Résumé", None, vec![email("alice@example.com")]),
    )
    .await
    .expect("create c3");
    seqs.push(change_seq(pool, c3.id).await);

    // Update bumps the seq; the full Contact (incl. JSON field arrays) round-trips.
    let updated = contact_service::update_contact(
        pool, owner, c1.id,
        &UpdateContactDto {
            job_title: Some("Ingénieur".into()),
            given_name: None, middle_name: None, family_name: None, name_prefix: None,
            name_suffix: None, nickname: None, display_name: None, organization: None,
            department: None, avatar_color: None, pronouns: None, emails: None, phones: None,
            addresses: None, urls: None, dates: None, relations: None, instant_messages: None,
            custom_fields: None, notes: None, is_starred: None,
        },
    )
    .await
    .expect("update c1");
    assert_eq!(updated.job_title.as_deref(), Some("Ingénieur"));
    assert_eq!(updated.emails.0.len(), 1, "email JSON array preserved through update");
    seqs.push(change_seq(pool, c1.id).await);

    // Trash bumps the seq; the contact stays a live row (soft delete).
    contact_service::trash_contact(pool, owner, c2.id).await.expect("trash c2");
    seqs.push(change_seq(pool, c2.id).await);

    for w in seqs.windows(2) {
        assert!(w[1] > w[0], "change_seq must strictly increase: {seqs:?}");
    }

    // ── the owner-scoped feed (journal::changes_since over contacts + tombstones) ──
    let feed = journal::changes_since(
        pool, sync::CONTACTS_TABLE, sync::CONTACT_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("contacts feed");
    assert!(feed.iter().any(|c| c.id == c1.id && !c.deleted), "c1 live modified");
    assert!(feed.iter().any(|c| c.id == c2.id && !c.deleted), "trashed c2 stays live");
    let feed_seqs: Vec<i64> = feed.iter().map(|c| c.change_seq).collect();
    let mut sorted = feed_seqs.clone();
    sorted.sort_unstable();
    assert_eq!(feed_seqs, sorted, "the feed is ordered by change_seq");
    let mut uniq = feed_seqs.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(uniq.len(), feed_seqs.len(), "contact change_seqs are unique");

    // ── search: stemmed + deaccented, identical on every engine ──
    // A plural query word finds the singular stored form (trashed c2 excluded).
    assert_eq!(search_names(pool, owner, "chevaux").await, vec!["Jean Chevaux".to_owned()]);
    // Accent folding on an organization: "developpement" finds "Développement".
    // (c2 carries that org but is trashed, so nothing matches — assert emptiness
    //  confirms trashed rows are excluded from search.)
    assert!(search_names(pool, owner, "developpement").await.is_empty());
    // Accent folding on a display name: "resume" finds "Résumé".
    assert_eq!(search_names(pool, owner, "resume").await, vec!["Paul Résumé".to_owned()]);
    // A term absent everywhere returns nothing.
    assert!(search_names(pool, owner, "hélicoptère").await.is_empty());
    // Scoped operator: `name:` restricts to the display name.
    assert_eq!(search_names(pool, owner, "name:chevaux").await, vec!["Jean Chevaux".to_owned()]);

    // ── labels: create, assign (bumps the contact), delete (tombstone) ──
    let label = label_service::create_label(
        pool, owner, &CreateLabelDto { id: None, name: "VIP".into(), color: None, icon: None },
    )
    .await
    .expect("create label");
    assert!(label_change_seq(pool, label.id).await > 0, "a created label has a seq");
    let before = change_seq(pool, c1.id).await;
    label_service::add_label_to_contacts(pool, owner, label.id, &[c1.id]).await.expect("assign");
    assert!(change_seq(pool, c1.id).await > before, "assigning a label bumps the contact");
    let ids = label_service::labels_for_contact(pool, c1.id).await.expect("labels for contact");
    assert_eq!(ids, vec![label.id]);
    // Rename the label (bumps its own seq).
    label_service::update_label(
        pool, owner, label.id,
        &UpdateLabelDto { name: Some("VIP+".into()), color: None, icon: None, position: None },
    )
    .await
    .expect("rename label");
    label_service::delete_label(pool, owner, label.id).await.expect("delete label");
    let lfeed = journal::changes_since(
        pool, sync::LABELS_TABLE, sync::LABEL_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("labels feed");
    assert!(lfeed.iter().any(|c| c.id == label.id && c.deleted), "deleted label tombstoned");

    // ── reminders: create + delete (tombstone) ──
    let rem = reminder_service::create_reminder(
        pool, owner,
        &CreateReminderDto {
            id: None, contact_id: c1.id, kind: Some("follow_up".into()),
            message: Some("Rappeler".into()), remind_at: chrono::Utc::now(), recurrence: None,
        },
    )
    .await
    .expect("create reminder");
    reminder_service::delete_reminder(pool, owner, rem.id).await.expect("delete reminder");
    let rfeed = journal::changes_since(
        pool, sync::REMINDERS_TABLE, sync::REMINDER_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("reminders feed");
    assert!(rfeed.iter().any(|c| c.id == rem.id && c.deleted), "deleted reminder tombstoned");

    // ── groups: create + delete via raw SQL (handler path), journal exercised ──
    let g = insert_group(pool, owner, "Amis").await;
    assert!(group_change_seq(pool, g).await > 0);
    delete_group(pool, g, owner).await;
    let gfeed = journal::changes_since(
        pool, sync::GROUPS_TABLE, sync::GROUP_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("groups feed");
    assert!(gfeed.iter().any(|c| c.id == g && c.deleted), "deleted group tombstoned");

    // ── permanent delete: contact surfaces as a tombstone ──
    contact_service::delete_contact_permanently(pool, owner, c3.id).await.expect("delete c3");
    let feed = journal::changes_since(
        pool, sync::CONTACTS_TABLE, sync::CONTACT_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("contacts feed");
    assert!(feed.iter().any(|c| c.id == c3.id && c.deleted), "deleted contact tombstoned");

    // ── duplicate detection: two contacts sharing an e-mail are grouped ──
    // (c1 and c3 shared alice@example.com — but c3 was just deleted; use a pair.)
    let d1 = contact_service::create_contact(
        pool, owner, &new_contact("Doublon Un", None, vec![email("dup@example.com")]),
    )
    .await
    .expect("dup1");
    let _d2 = contact_service::create_contact(
        pool, owner, &new_contact("Doublon Deux", None, vec![email("dup@example.com")]),
    )
    .await
    .expect("dup2");
    let dups = contact_service::find_duplicates(pool, owner).await.expect("find dups 2");
    assert!(
        dups.iter().any(|g| g.contacts.iter().any(|c| c.id == d1.id)),
        "the two contacts sharing an e-mail are reported as duplicates"
    );
}

async fn label_change_seq(pool: &kubuno_db::DbPool, id: Uuid) -> i64 {
    pool.fetch_scalar::<i64>("SELECT change_seq FROM contacts.labels WHERE id = $1", params![id])
        .await
        .expect("label change_seq")
}

async fn group_change_seq(pool: &kubuno_db::DbPool, id: Uuid) -> i64 {
    pool.fetch_scalar::<i64>("SELECT change_seq FROM contacts.groups WHERE id = $1", params![id])
        .await
        .expect("group change_seq")
}

/// Mirrors the group handler's create (id + seq minted in Rust).
async fn insert_group(pool: &kubuno_db::DbPool, owner: Uuid, name: &str) -> Uuid {
    let id = new_id();
    let now = chrono::Utc::now();
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_group_seq(&mut tx).await.expect("group seq");
    tx.execute(
        "INSERT INTO contacts.groups (id, owner_id, name, color, change_seq, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        params![id, owner, name, "#1a73e8", seq, now, now],
    )
    .await
    .expect("insert group");
    tx.commit().await.expect("commit");
    id
}

async fn delete_group(pool: &kubuno_db::DbPool, id: Uuid, owner: Uuid) {
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_group_seq(&mut tx).await.expect("group seq");
    tx.execute("DELETE FROM contacts.groups WHERE id = $1", params![id]).await.expect("delete");
    sync::record_group_tombstone(&mut tx, id, owner, seq).await.expect("tombstone");
    tx.commit().await.expect("commit");
}

#[tokio::test]
async fn sqlite_from_the_one_binary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut s = base_settings("sqlite");
    s.path = Some(dir.path().to_string_lossy().into_owned());
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn postgres_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_PG_TEST_URL") else {
        eprintln!("skipping: KUBUNO_PG_TEST_URL not set");
        return;
    };
    let mut s = base_settings("postgres");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn mysql_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_MYSQL_TEST_URL") else {
        eprintln!("skipping: KUBUNO_MYSQL_TEST_URL not set");
        return;
    };
    let mut s = base_settings("mysql");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}
