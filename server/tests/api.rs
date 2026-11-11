//! End-to-end API tests against the real question bank and a temp-file
//! SQLite database, driving the router directly (no network).

use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use gedpasser_server::{App, Config, build_app};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

fn content_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("content")
}

fn temp_db(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("server-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join(format!("{name}.db"));
    for ext in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{ext}", db.display()));
    }
    db
}

async fn app(db: &Path, serve_drafts: bool) -> App {
    build_app(&Config { content_dir: content_dir(), db_path: db.to_path_buf(), serve_drafts })
        .await
        .expect("app builds")
}

/// Send a request; returns status, JSON body (or Null) and any session cookie set.
async fn call(router: &Router, method: &str, uri: &str, cookie: Option<&str>, body: Option<Value>) -> (StatusCode, Value, Option<String>) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let req = match body {
        Some(b) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let set_cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json, set_cookie)
}

async fn create(router: &Router, name: &str, secret: &str) -> (Value, String) {
    let (status, body, cookie) =
        call(router, "POST", "/api/profiles", None, Some(json!({"display_name": name, "secret": secret}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    (body, cookie.expect("session cookie"))
}

#[tokio::test]
async fn profile_login_logout() {
    let db = temp_db("auth");
    let app = app(&db, false).await;
    let r = &app.router;

    let (me, cookie) = create(r, "Ada", "1234").await;
    assert_eq!(me["display_name"], "Ada");
    let id = me["id"].as_i64().unwrap();

    let (status, body, _) = call(r, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["display_name"], "Ada");

    // Duplicate names (any case) and short secrets are refused.
    let (status, _, _) = call(r, "POST", "/api/profiles", None, Some(json!({"display_name": "ada", "secret": "9999"}))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _, _) = call(r, "POST", "/api/profiles", None, Some(json!({"display_name": "Bo", "secret": "12"}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Profile picker lists names only.
    let (_, list, _) = call(r, "GET", "/api/profiles", None, None).await;
    assert_eq!(list, json!([{"id": id, "display_name": "Ada"}]));

    // Wrong secret and unknown profile both give 401.
    let (status, _, c) = call(r, "POST", "/api/login", None, Some(json!({"user_id": id, "secret": "0000"}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(c.is_none());
    let (status, _, _) = call(r, "POST", "/api/login", None, Some(json!({"user_id": 999, "secret": "1234"}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Logout ends the session.
    let (status, _, cleared) = call(r, "POST", "/api/logout", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(cleared.as_deref(), Some("ged_session="));
    let (status, _, _) = call(r, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Correct secret logs back in with a new session.
    let (status, _, c) = call(r, "POST", "/api/login", None, Some(json!({"user_id": id, "secret": "1234"}))).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = call(r, "GET", "/api/me", Some(&c.unwrap()), None).await;
    assert_eq!(status, StatusCode::OK);

    // Stored hash is Argon2id; the raw secret is nowhere in the row.
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert!(hash.starts_with("$argon2id$"), "{hash}");
    assert!(!hash.contains("1234"));
    // Only token hashes are stored, never the cookie value.
    let token = cookie.trim_start_matches("ged_session=");
    let stored: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE token_hash = ?")
        .bind(token)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(stored, 0);
}

#[tokio::test]
async fn practice_requires_login() {
    let db = temp_db("nologin");
    let app = app(&db, true).await;
    for (method, uri) in [("GET", "/api/skills"), ("GET", "/api/practice/next?skill=q-number-ops"), ("GET", "/api/me")] {
        let (status, _, _) = call(&app.router, method, uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    let (status, _, _) = call(
        &app.router,
        "POST",
        "/api/answer",
        Some("ged_session=bogus"),
        Some(json!({"question_id": "q-number-ops-0001", "response": "1", "time_ms": 5})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// A temp copy of the real content with every question and lesson set back
/// to draft, for tests about how drafts are handled.
fn draft_content(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("server-tests").join(format!("{name}-content"));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&content_dir(), &dir);
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("math")).unwrap().map(|e| e.unwrap().path()).collect();
    files.push(dir.join("principles.toml"));
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap().replace("status = \"reviewed\"", "status = \"draft\"");
        std::fs::write(&f, text).unwrap();
    }
    dir
}

async fn app_with(content: &Path, db: &Path, serve_drafts: bool) -> App {
    build_app(&Config { content_dir: content.to_path_buf(), db_path: db.to_path_buf(), serve_drafts })
        .await
        .expect("app builds")
}

#[tokio::test]
async fn drafts_only_served_with_flag() {
    let db = temp_db("drafts");
    let content = draft_content("drafts");
    let app = app_with(&content, &db, false).await;
    let (_, cookie) = create(&app.router, "Cy", "abcd").await;
    let (status, body, _) = call(&app.router, "GET", "/api/practice/next?skill=q-number-ops", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["question"], Value::Null, "drafts must not be served without the flag");
    let (_, skills, _) = call(&app.router, "GET", "/api/skills", Some(&cookie), None).await;
    assert!(skills.as_array().unwrap().iter().all(|s| s["available"] == 0));
    let (status, _, _) = call(
        &app.router,
        "POST",
        "/api/answer",
        Some(&cookie),
        Some(json!({"question_id": "q-number-ops-0001", "response": "26.35", "time_ms": 5})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    app.pool.close().await;

    let app = app_with(&content, &db, true).await;
    let (status, body, _) = call(&app.router, "GET", "/api/practice/next?skill=q-number-ops", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    let q = &body["question"];
    assert_eq!(q["draft"], true);
    for hidden in ["answer", "check", "explanation", "common_mistake", "answer_value"] {
        assert!(q.get(hidden).is_none(), "question leaks {hidden}");
    }
    let (status, _, _) = call(&app.router, "GET", "/api/practice/next?skill=not-a-skill", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn principles_and_lessons() {
    let db = temp_db("principles");
    let content = draft_content("principles");
    let app = app_with(&content, &db, false).await;
    let (_, cookie) = create(&app.router, "Eve", "pw12").await;

    // Without drafts: every principle is listed, but draft lessons are withheld.
    let (status, list, _) = call(&app.router, "GET", "/api/principles", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 62);
    assert_eq!(list[0]["skill"], "q-number-ops", "listed in skill order");
    assert!(list.iter().all(|p| p["has_lesson"] == false && !p["name"].as_str().unwrap().is_empty()));
    let (status, lesson, _) = call(&app.router, "GET", "/api/principles/Q.3.d", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(lesson["lesson"], Value::Null);
    assert_eq!(lesson["skill_name"], "Ratios, rates, percents and scale");
    let (status, _, _) = call(&app.router, "GET", "/api/principles/Q.9.z", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = call(&app.router, "GET", "/api/principles", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    app.pool.close().await;

    // With drafts: lessons are served and marked as drafts.
    let app = app_with(&content, &db, true).await;
    let (_, lesson, _) = call(&app.router, "GET", "/api/principles/Q.3.d", Some(&cookie), None).await;
    assert!(lesson["lesson"].as_str().unwrap().contains("Percent"));
    assert_eq!(lesson["draft"], true);

    // Questions and results carry their principle; hint use and principle are saved.
    let (_, next, _) = call(&app.router, "GET", "/api/practice/next?skill=q-ratios-percent", Some(&cookie), None).await;
    let q = &next["question"];
    assert!(q["principle"]["id"].as_str().unwrap().starts_with("Q.3."));
    assert!(!q["principle"]["name"].as_str().unwrap().is_empty());
    let (_, res, _) = call(
        &app.router,
        "POST",
        "/api/answer",
        Some(&cookie),
        Some(json!({"question_id": "q-ratios-percent-0002", "response": "88.2", "time_ms": 900, "hint_used": true})),
    )
    .await;
    assert_eq!(res["correct"], true);
    assert_eq!(res["principle"]["id"], "Q.3.d");
    let row: (String, i64) =
        sqlx::query_as("SELECT principle_id, hint_used FROM attempts WHERE question_id = 'q-ratios-percent-0002'")
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(row, ("Q.3.d".to_string(), 1));

    // Per-principle progress shows up on the list.
    let (_, list, _) = call(&app.router, "GET", "/api/principles", Some(&cookie), None).await;
    let p = list.as_array().unwrap().iter().find(|p| p["id"] == "Q.3.d").unwrap();
    assert_eq!((p["answered"].as_u64(), p["correct"].as_u64(), p["has_lesson"].as_bool()), (Some(1), Some(1), Some(true)));
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let p = entry.unwrap().path();
        let dest = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_dir(&p, &dest);
        } else {
            std::fs::copy(&p, &dest).unwrap();
        }
    }
}

#[tokio::test]
async fn flags_remove_questions_until_resolved() {
    // Work on a copy of the content so we can edit a question.
    let content = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("server-tests").join("flag-content");
    let _ = std::fs::remove_dir_all(&content);
    copy_dir(&content_dir(), &content);
    let db = temp_db("flags");
    let config = Config { content_dir: content.clone(), db_path: db.clone(), serve_drafts: true };
    let app = build_app(&config).await.unwrap();
    let (_, cookie) = create(&app.router, "Fay", "flag1").await;

    let flag = json!({"question_id": "q-number-ops-0001", "reason": "wrong_answer", "note": "  I think it's 26.30  "});
    let (status, _, _) = call(&app.router, "POST", "/api/flags", None, Some(flag.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = call(&app.router, "POST", "/api/flags", Some(&cookie), Some(flag)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = call(
        &app.router,
        "POST",
        "/api/flags",
        Some(&cookie),
        Some(json!({"question_id": "nope-0001", "reason": "typo"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The flagged question is never served, and the count drops by one.
    let (_, skills, _) = call(&app.router, "GET", "/api/skills", Some(&cookie), None).await;
    let ops = skills.as_array().unwrap().iter().find(|s| s["id"] == "q-number-ops").unwrap()["available"].as_u64().unwrap();
    let total = std::fs::read_to_string(content.join("math").join("q-number-ops.toml")).unwrap().matches("[[question]]").count() as u64;
    assert_eq!(ops, total - 1);
    for _ in 0..30 {
        let (_, next, _) = call(&app.router, "GET", "/api/practice/next?skill=q-number-ops", Some(&cookie), None).await;
        assert_ne!(next["question"]["id"], "q-number-ops-0001");
    }

    let open = gedpasser_server::flags::list_open(&app.pool).await.unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].note, "I think it's 26.30");
    app.pool.close().await;

    // Editing the question settles the flag on the next start. An edited
    // question must go back to draft (the review gate rejects an edited
    // `reviewed` item), as a real fix would.
    let file = content.join("math").join("q-number-ops.toml");
    let text = std::fs::read_to_string(&file)
        .unwrap()
        .replace("How many dollars do they spend in all?", "How many dollars do they spend altogether?")
        .replacen("status = \"reviewed\"", "status = \"draft\"", 1); // q-number-ops-0001 is the first question
    std::fs::write(&file, text).unwrap();
    let app = build_app(&config).await.unwrap();
    assert!(gedpasser_server::flags::list_open(&app.pool).await.unwrap().is_empty());
    let (_, skills, _) = call(&app.router, "GET", "/api/skills", Some(&cookie), None).await;
    let ops = skills.as_array().unwrap().iter().find(|s| s["id"] == "q-number-ops").unwrap()["available"].as_u64().unwrap();
    assert_eq!(ops, total);

    // A flag can also be dismissed by hand.
    call(&app.router, "POST", "/api/flags", Some(&cookie), Some(json!({"question_id": "q-number-ops-0002", "reason": "typo"}))).await;
    let id = gedpasser_server::flags::list_open(&app.pool).await.unwrap()[0].id;
    assert!(gedpasser_server::flags::dismiss(&app.pool, id).await.unwrap());
    assert!(!gedpasser_server::flags::dismiss(&app.pool, id).await.unwrap(), "already resolved");
    assert!(gedpasser_server::flags::list_open(&app.pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn answers_graded_saved_and_survive_restart() {
    let db = temp_db("restart");
    let app = app(&db, true).await;
    let (me, cookie) = create(&app.router, "Dee", "pass1").await;
    let id = me["id"].as_i64().unwrap();

    // Fill-in: "26.350" matches the stored "26.35" numerically.
    let (status, res, _) = call(
        &app.router,
        "POST",
        "/api/answer",
        Some(&cookie),
        Some(json!({"question_id": "q-number-ops-0001", "response": "26.350", "time_ms": 4200})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["correct"], true);
    assert_eq!(res["answer"], "26.35");
    assert!(res["explanation"].as_str().unwrap().contains("26.35"));

    // Multiple choice, wrong.
    let (_, res, _) = call(
        &app.router,
        "POST",
        "/api/answer",
        Some(&cookie),
        Some(json!({"question_id": "q-number-ops-0002", "response": "60", "time_ms": 3000})),
    )
    .await;
    assert_eq!(res["correct"], false);
    assert_eq!(res["answer"], "6");
    assert!(!res["common_mistake"].as_str().unwrap().is_empty());

    // The next question prefers one never answered.
    let (_, next, _) = call(&app.router, "GET", "/api/practice/next?skill=q-number-ops", Some(&cookie), None).await;
    let next_id = next["question"]["id"].as_str().unwrap();
    assert!(next_id != "q-number-ops-0001" && next_id != "q-number-ops-0002", "{next_id}");

    let rows: Vec<(String, i64, i64)> =
        sqlx::query_as("SELECT question_id, correct, time_ms FROM attempts WHERE user_id = ? ORDER BY id")
            .bind(id)
            .fetch_all(&app.pool)
            .await
            .unwrap();
    assert_eq!(rows, vec![("q-number-ops-0001".into(), 1, 4200), ("q-number-ops-0002".into(), 0, 3000)]);

    // "Restart": drop everything and reopen the same database file.
    app.pool.close().await;
    drop(app);
    let app = self::app(&db, true).await;

    let (status, _, _) = call(&app.router, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK, "session survives restart");
    let (_, skills, _) = call(&app.router, "GET", "/api/skills", Some(&cookie), None).await;
    let ops = skills.as_array().unwrap().iter().find(|s| s["id"] == "q-number-ops").unwrap();
    assert_eq!(ops["answered"], 2);
    assert_eq!(ops["correct"], 1);
    let bank = ged_content::check_content(&content_dir());
    let ops_questions: Vec<&ged_content::Question> =
        bank.questions.iter().map(|(_, q)| q).filter(|q| q.skill == "q-number-ops").collect();
    assert_eq!(ops["available"].as_u64(), Some(ops_questions.len() as u64));

    // Once everything else has been answered correctly, the missed question
    // (0002) comes before any question answered correctly.
    let mut last = String::new();
    for q in ops_questions.iter().filter(|q| q.id != "q-number-ops-0001" && q.id != "q-number-ops-0002") {
        let (_, res, _) = call(
            &app.router,
            "POST",
            "/api/answer",
            Some(&cookie),
            Some(json!({"question_id": q.id, "response": q.answer, "time_ms": 100})),
        )
        .await;
        assert_eq!(res["correct"], true, "{} graded wrong with its own answer", q.id);
        last = q.id.clone();
    }
    let (_, next, _) =
        call(&app.router, "GET", &format!("/api/practice/next?skill=q-number-ops&after={last}"), Some(&cookie), None).await;
    assert_eq!(next["question"]["id"], "q-number-ops-0002");
}


/// Answer `n` questions of a skill, right or wrong, using the bank's keys.
async fn answer_skill(router: &Router, cookie: &str, skill: &str, n: usize, right: bool) {
    let bank = ged_content::check_content(&content_dir());
    let qs: Vec<&ged_content::Question> = bank.questions.iter().map(|(_, q)| q).filter(|q| q.skill == skill).take(n).collect();
    for q in qs {
        let response = if right { q.answer.clone() } else { "999999".to_string() };
        let (status, res, _) = call(
            router,
            "POST",
            "/api/answer",
            Some(cookie),
            Some(json!({"question_id": q.id, "response": response, "time_ms": 1000, "mode": "skill"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{res}");
        assert_eq!(res["correct"], right, "{}", q.id);
    }
}

#[tokio::test]
async fn mastery_focus_and_practice_modes() {
    let db = temp_db("phase3");
    let app = app(&db, false).await;
    let r = &app.router;
    let (_, cookie) = create(r, "Gus", "math").await;

    // Before any answers: every skill is not started; focus still suggests three.
    let (status, prog, _) = call(r, "GET", "/api/progress", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    let skills = prog["skills"].as_array().unwrap();
    assert!(skills.iter().all(|s| s["level"] == "not_started" && s["mastery"].is_null()));
    assert_eq!(prog["focus"].as_array().unwrap().len(), 3);

    // Strong in number ops, weak in probability.
    answer_skill(r, &cookie, "q-number-ops", 6, true).await;
    answer_skill(r, &cookie, "q-probability", 6, false).await;
    let (_, prog, _) = call(r, "GET", "/api/progress", Some(&cookie), None).await;
    let by_id = |id: &str| prog["skills"].as_array().unwrap().iter().find(|s| s["id"] == id).unwrap().clone();
    let ops = by_id("q-number-ops");
    let prob = by_id("q-probability");
    assert!(ops["mastery"].as_f64().unwrap() > 0.8, "{ops}");
    assert_eq!(ops["level"], "strong");
    assert!(prob["mastery"].as_f64().unwrap() < 0.2, "{prob}");
    assert_eq!(prob["level"], "needs_work");
    let focus: Vec<&str> = prog["focus"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(focus[0], "q-probability", "weakest first: {focus:?}");
    assert!(!focus.contains(&"q-number-ops"));

    // Principle mastery shows up too.
    let (_, principles, _) = call(r, "GET", "/api/principles", Some(&cookie), None).await;
    let q8b = principles.as_array().unwrap().iter().find(|p| p["id"] == "Q.8.b").unwrap();
    assert_eq!(q8b["level"], "needs_work");
    assert!(q8b["available"].as_u64().unwrap() > 0);

    // Principle practice serves only that principle.
    for _ in 0..15 {
        let (status, next, _) = call(r, "GET", "/api/practice/next?principle=Q.3.d", Some(&cookie), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(next["question"]["principle"]["id"], "Q.3.d");
    }

    // Mixed practice leans toward the weak skill: probability (mastery < 0.2)
    // should come up far more often than number ops (> 0.8).
    let (mut prob_n, mut ops_n) = (0, 0);
    for _ in 0..300 {
        let (_, next, _) = call(r, "GET", "/api/practice/next?mode=mixed", Some(&cookie), None).await;
        match next["question"]["skill"].as_str().unwrap() {
            "q-probability" => prob_n += 1,
            "q-number-ops" => ops_n += 1,
            _ => {}
        }
        assert!(!next["question"]["skill_name"].as_str().unwrap().is_empty());
    }
    assert!(prob_n > 2 * ops_n, "probability {prob_n} vs number ops {ops_n}");

    // Mode is stored with the attempt.
    let (_, next, _) = call(r, "GET", "/api/practice/next?principle=Q.4.e", Some(&cookie), None).await;
    let qid = next["question"]["id"].as_str().unwrap().to_string();
    call(r, "POST", "/api/answer", Some(&cookie), Some(json!({"question_id": qid, "response": "1", "time_ms": 5, "mode": "principle"}))).await;
    let mode: String = sqlx::query_scalar("SELECT mode FROM attempts ORDER BY id DESC LIMIT 1").fetch_one(&app.pool).await.unwrap();
    assert_eq!(mode, "principle");

    // Bad scopes.
    for uri in ["/api/practice/next?principle=Q.9.z", "/api/practice/next?skill=nope"] {
        let (status, _, _) = call(r, "GET", uri, Some(&cookie), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
    let (status, _, _) = call(r, "GET", "/api/practice/next", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[test]
fn weighted_pick_and_focus_order() {
    use gedpasser_server::api::pick_weighted;
    let w = vec![("a".to_string(), 1.0), ("b".to_string(), 3.0)];
    assert_eq!(pick_weighted(&w, 0.0).as_deref(), Some("a"));
    assert_eq!(pick_weighted(&w, 0.24).as_deref(), Some("a"));
    assert_eq!(pick_weighted(&w, 0.26).as_deref(), Some("b"));
    assert_eq!(pick_weighted(&w, 1.0).as_deref(), Some("b"));
    assert_eq!(pick_weighted(&[], 0.5), None);
}