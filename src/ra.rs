// the retroachievements api integration!!!
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::Mutex;

#[allow(dead_code)]
pub const RC_CONSOLE_NINTENDO: u32 = 7;

pub const RC_CLIENT_EVENT_ACHIEVEMENT_TRIGGERED: u32 = 1;
pub const RC_CLIENT_EVENT_RESET: u32 = 14;

const RC_CLIENT_CATEGORY_CORE: i32 = 1;
const RC_CLIENT_ACHIEVEMENT_LIST_GROUPING_LOCK_STATE: i32 = 0;
const RC_CLIENT_ACHIEVEMENT_STATE_UNLOCKED: u8 = 2;
const RC_CLIENT_ACHIEVEMENT_STATE_DISABLED: u8 = 3;

const HASH_BUF_SIZE: usize = 512;

unsafe extern "C" {
    fn rc_hash_initialize_iterator(
        iterator: *mut u8,
        path: *const c_char,
        buffer: *const u8,
        buffer_size: usize,
    );
    fn rc_hash_iterate(hash: *mut u8, iterator: *mut u8) -> i32;
    fn rc_hash_destroy_iterator(iterator: *mut u8);

    fn rc_client_create(
        read_memory_function: unsafe extern "C" fn(u32, *mut u8, u32, *mut rc_client_opaque) -> u32,
        server_call_function: unsafe extern "C" fn(*const rc_api_request_t, RcServerCallback, *mut c_void, *mut rc_client_opaque),
    ) -> *mut rc_client_opaque;
    fn rc_client_destroy(client: *mut rc_client_opaque);
    fn rc_client_set_hardcore_enabled(client: *mut rc_client_opaque, enabled: i32);
#[allow(dead_code)]
fn rc_client_get_hardcore_enabled(client: *const rc_client_opaque) -> i32;
    fn rc_client_set_event_handler(client: *mut rc_client_opaque, handler: RcEventHandler);
    fn rc_client_begin_login_with_password(
        client: *mut rc_client_opaque,
        username: *const c_char,
        password: *const c_char,
        callback: RcClientCallback,
        callback_userdata: *mut c_void,
    ) -> *mut c_void;
    fn rc_client_begin_login_with_token(
        client: *mut rc_client_opaque,
        username: *const c_char,
        token: *const c_char,
        callback: RcClientCallback,
        callback_userdata: *mut c_void,
    ) -> *mut c_void;
    fn rc_client_logout(client: *mut rc_client_opaque);
    fn rc_client_begin_load_game(
        client: *mut rc_client_opaque,
        hash: *const c_char,
        callback: RcClientCallback,
        callback_userdata: *mut c_void,
    ) -> *mut c_void;
    fn rc_client_unload_game(client: *mut rc_client_opaque);
    fn rc_client_is_game_loaded(client: *const rc_client_opaque) -> i32;
    fn rc_client_do_frame(client: *mut rc_client_opaque);
    fn rc_client_idle(client: *mut rc_client_opaque);
    fn rc_client_get_user_info(client: *const rc_client_opaque) -> *const RcClientUser;
    fn rc_client_get_game_info(client: *const rc_client_opaque) -> *const RcClientGame;
    fn rc_client_create_achievement_list(
        client: *mut rc_client_opaque,
        category: i32,
        grouping: i32,
    ) -> *mut RcClientAchievementList;
    fn rc_client_destroy_achievement_list(list: *mut RcClientAchievementList);
}

#[repr(C)]
pub struct rc_client_opaque {
    _private: [u8; 0],
}

#[repr(C)]
struct rc_api_request_t {
    url: *const c_char,
    post_data: *const c_char,
    content_type: *const c_char,
    buffer: [u8; 64],
}

type RcServerCallback = unsafe extern "C" fn(*const RcApiServerResponse, *mut c_void);
type RcClientCallback = unsafe extern "C" fn(i32, *const c_char, *mut rc_client_opaque, *mut c_void);
type RcEventHandler = unsafe extern "C" fn(*const RcClientEvent, *mut rc_client_opaque);

#[repr(C)]
struct RcApiServerResponse {
    body: *const c_char,
    body_length: usize,
    http_status_code: i32,
}

#[repr(C)]
struct RcClientUser {
    display_name: *const c_char,
    username: *const c_char,
    token: *const c_char,
    score: u32,
    score_softcore: u32,
    num_unread_messages: u32,
    avatar_url: *const c_char,
    avatar_last_updated: i64,
}


#[repr(C)]
struct RcClientGame {
    id: u32,
    console_id: u32,
    title: *const c_char,
    hash: *const c_char,
    badge_name: *const c_char,
    badge_url: *const c_char,
}

#[repr(C)]
struct RcClientAchievementBucket {
    achievements: *const *const RcClientAchievement,
    num_achievements: u32,
    label: *const c_char,
    subset_id: u32,
    bucket_type: u8,
}

#[repr(C)]
struct RcClientAchievementList {
    buckets: *const RcClientAchievementBucket,
    num_buckets: u32,
}

#[repr(C)]
struct RcClientAchievement {
    title: *const c_char,
    description: *const c_char,
    badge_name: [u8; 8],
    measured_progress: [u8; 24],
    measured_percent: f32,
    id: u32,
    points: u32,
    unlock_time: i64,
    state: u8,
    category: u8,
    bucket: u8,
    unlocked: u8,
    rarity: f32,
    rarity_hardcore: f32,
    kind: u8,
}

#[repr(C)]
struct RcClientEvent {
    kind: u32,
    achievement: *mut RcClientAchievement,
    leaderboard: *mut c_void,
    leaderboard_tracker: *mut c_void,
    leaderboard_scoreboard: *mut c_void,
    server_error: *mut c_void,
    subset: *mut c_void,
}

static EMU_THREAD_PTR: AtomicU64 = AtomicU64::new(0);

pub fn set_emu_thread_ptr(ptr: u64) {
    EMU_THREAD_PTR.store(ptr, Ordering::Relaxed);
}

pub fn clear_emu_thread_ptr() {
    EMU_THREAD_PTR.store(0, Ordering::Relaxed);
}

unsafe extern "C" fn read_memory_callback(
    address: u32,
    buffer: *mut u8,
    num_bytes: u32,
    _client: *mut rc_client_opaque,
) -> u32 {
    let emu = EMU_THREAD_PTR.load(Ordering::Relaxed) as *const Emulator;
    if emu.is_null() || buffer.is_null() {
        return 0;
    }
    let emu = &*emu;

    let addr = address as usize;
    let num = num_bytes as usize;
    match addr {
        0x0000..=0x1FFF => {
            let base = addr & 0x7FF;
            let ram = &emu.ram;
            for i in 0..num {
                *buffer.add(i) = ram[(base + i) & 0x7FF];
            }
            num as u32
        }
        0x6000..=0x7FFF => {
            let cart = match emu.cart.as_ref() {
                Some(c) => c,
                None => return 0,
            };
            if cart.prg_ram.is_empty() {
                return 0;
            }
            let ram_len = cart.prg_ram.len();
            let offset = (addr - 0x6000) % ram_len;
            for i in 0..num {
                let idx = (offset + i) % ram_len;
                *buffer.add(i) = cart.prg_ram[idx];
            }
            num as u32
        }
        _ => 0,
    }
}

unsafe extern "C" fn server_call_callback(
    request: *const rc_api_request_t,
    callback: RcServerCallback,
    callback_data: *mut c_void,
    _client: *mut rc_client_opaque,
) {
    let req = &*request;
    let url = if req.url.is_null() {
        String::new()
    } else {
        CStr::from_ptr(req.url).to_string_lossy().into_owned()
    };
    let post_data = if req.post_data.is_null() {
        None
    } else {
        Some(CStr::from_ptr(req.post_data).to_string_lossy().into_owned())
    };

    let callback_data_addr = callback_data as usize;

    std::thread::spawn(move || {
        let callback_data = callback_data_addr as *mut c_void;
        let (status_code, body) = match perform_http(&url, post_data.as_deref()) {
            Ok(b) => (200, b),
            Err(e) => {
                eprintln!("[RA] HTTP error for {}: {}", url, e);
                (-1, Vec::new())
            }
        };

        let resp = RcApiServerResponse {
            body: body.as_ptr() as *const c_char,
            body_length: body.len(),
            http_status_code: status_code,
        };
        callback(&resp, callback_data);
    });
}

fn perform_http(url: &str, post_data: Option<&str>) -> Result<Vec<u8>, String> {
    let tls = match ureq::native_tls::TlsConnector::new() {
        Ok(c) => c,
        Err(e) => return Err(format!("TLS init failed: {}", e)),
    };
    let agent = ureq::AgentBuilder::new()
        .tls_connector(std::sync::Arc::new(tls))
        .timeout(std::time::Duration::from_secs(15))
        .build();
    let result = match post_data {
        Some(data) => agent
            .post(url)
            .set("Content-Type", "application/x-www-form-urlencoded")
            .send_string(data),
        None => agent.get(url).call(),
    };

    match result {
        Ok(resp) => {
            let mut bytes = Vec::new();
            use std::io::Read;
            resp.into_reader()
                .take(4 * 1024 * 1024)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            Ok(bytes)
        }
        Err(ureq::Error::Status(_code, resp)) => {
            let mut bytes = Vec::new();
            use std::io::Read;
            let _ = resp
                .into_reader()
                .take(1024 * 1024)
                .read_to_end(&mut bytes);
            Ok(bytes)
        }
        Err(e) => Err(e.to_string()),
    }
}

unsafe extern "C" fn event_handler(event: *const RcClientEvent, _client: *mut rc_client_opaque) {
    let event = &*event;
    match event.kind {
        RC_CLIENT_EVENT_ACHIEVEMENT_TRIGGERED => {
            if !event.achievement.is_null() {
                let a = &*event.achievement;
                let title = cstr_lossy(a.title);
                let desc = cstr_lossy(a.description);
                let points = a.points;
                let hardcore = HARDCORE_ACTIVE.load(Ordering::Relaxed);
                println!(
                    "[RA] Achievement unlocked: {} ({}) — {} points, {} mode",
                    title,
                    desc,
                    points,
                    if hardcore { "hardcore" } else { "softcore" }
                );
                TOASTS.lock().unwrap().push(RaToast {
                    title,
                    description: desc,
                    points,
                    hardcore,
                });
            }
        }
        RC_CLIENT_EVENT_RESET => {
            RESET_REQUESTED.store(true, Ordering::Relaxed);
            println!("[RA] Reset requested (hardcore rules violated)");
        }
        _ => {}
    }
}

pub struct RaToast {
    pub title: String,
    pub description: String,
    pub points: u32,
    pub hardcore: bool,
}

static TOASTS: Mutex<Vec<RaToast>> = Mutex::new(Vec::new());

pub fn take_toasts() -> Vec<RaToast> {
    std::mem::take(&mut TOASTS.lock().unwrap())
}

pub struct RaAchievement {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub points: u32,
    pub unlocked: bool,
    pub disabled: bool,
    pub measured_progress: String,
}

fn cstr_fixed(p: &[u8]) -> String {
    let end = p.iter().position(|&b| b == 0).unwrap_or(p.len());
    String::from_utf8_lossy(&p[..end]).into_owned()
}

pub fn get_achievements() -> Vec<RaAchievement> {
    let client = client_ptr();
    if client.is_null() || !game_loaded() {
        return Vec::new();
    }
    unsafe {
        let list = rc_client_create_achievement_list(
            client,
            RC_CLIENT_CATEGORY_CORE,
            RC_CLIENT_ACHIEVEMENT_LIST_GROUPING_LOCK_STATE,
        );
        if list.is_null() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let l = &*list;
        for b in 0..l.num_buckets as usize {
            let bucket = &*l.buckets.add(b);
            for a in 0..bucket.num_achievements as usize {
                let ap = *bucket.achievements.add(a);
                if ap.is_null() {
                    continue;
                }
                let a = &*ap;
                out.push(RaAchievement {
                    id: a.id,
                    title: cstr_lossy(a.title),
                    description: cstr_lossy(a.description),
                    points: a.points,
                    unlocked: a.state == RC_CLIENT_ACHIEVEMENT_STATE_UNLOCKED,
                    disabled: a.state == RC_CLIENT_ACHIEVEMENT_STATE_DISABLED,
                    measured_progress: cstr_fixed(&a.measured_progress),
                });
            }
        }
        rc_client_destroy_achievement_list(list);
        out
    }
}

unsafe fn cstr_lossy(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

unsafe extern "C" fn login_callback(
    result: i32,
    error_message: *const c_char,
    _client: *mut rc_client_opaque,
    userdata: *mut c_void,
) {
    let slot = userdata as *const LoginResultSlot;
    if !slot.is_null() {
        (*slot).0.lock().unwrap().replace(LoginOutcome {
            result,
            error: if error_message.is_null() {
                None
            } else {
                Some(CStr::from_ptr(error_message).to_string_lossy().into_owned())
            },
        });
    }
}

pub struct LoginOutcome {
    pub result: i32,
    pub error: Option<String>,
}

pub struct LoginResultSlot(pub Mutex<Option<LoginOutcome>>);
pub struct LoadResultSlot(pub Mutex<Option<LoginOutcome>>);

unsafe extern "C" fn load_game_callback(
    result: i32,
    error_message: *const c_char,
    _client: *mut rc_client_opaque,
    userdata: *mut c_void,
) {
    let slot = userdata as *const LoadResultSlot;
    if !slot.is_null() {
        (*slot).0.lock().unwrap().replace(LoginOutcome {
            result,
            error: if error_message.is_null() {
                None
            } else {
                Some(CStr::from_ptr(error_message).to_string_lossy().into_owned())
            },
        });
    }
}

static CLIENT: AtomicPtr<rc_client_opaque> = AtomicPtr::new(std::ptr::null_mut());
static HARDCORE_ACTIVE: AtomicBool = AtomicBool::new(false);
static RESET_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn hardcore_active() -> bool {
    HARDCORE_ACTIVE.load(Ordering::Relaxed)
}

pub fn hardcore_requested() -> bool {
    crate::config::load_ra_hardcore()
}

pub fn init_client() {
    if !CLIENT.load(Ordering::Acquire).is_null() {
        return;
    }
    unsafe {
        let client = rc_client_create(read_memory_callback, server_call_callback);
        if client.is_null() {
            eprintln!("[RA] failed to create rc_client");
            return;
        }
        rc_client_set_event_handler(client, event_handler);

        let hardcore = hardcore_requested();
        rc_client_set_hardcore_enabled(client, hardcore as i32);
        HARDCORE_ACTIVE.store(hardcore, Ordering::Relaxed);

        CLIENT.store(client, Ordering::Release);

        if let (Some(username), Some(token)) = (
            crate::config::load_ra_username(),
            crate::config::load_ra_token(),
        ) {
            if !username.is_empty() && !token.is_empty() {
                if let Ok(u) = CString::new(username.clone()) {
                    if let Ok(t) = CString::new(token) {
                        rc_client_begin_login_with_token(
                            client,
                            u.as_ptr(),
                            t.as_ptr(),
                            login_callback,
                            std::ptr::null_mut(),
                        );
                        println!("[RA] logging in as {} (token)", username);
                    }
                }
            }
        }
    }
}

pub fn client_ptr() -> *mut rc_client_opaque {
    CLIENT.load(Ordering::Acquire)
}

#[allow(dead_code)]
pub fn client_exists() -> bool {
    !client_ptr().is_null()
}

pub fn shutdown() {
    let client = CLIENT.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if !client.is_null() {
        unsafe { rc_client_destroy(client) };
    }
}

pub fn login(username: &str, password: &str) -> Result<(), String> {
    let client = client_ptr();
    if client.is_null() {
        return Err("RetroAchievements client not initialized".to_string());
    }
    unsafe {
        let slot = LoginResultSlot(Mutex::new(None));
        let u = CString::new(username).map_err(|_| "invalid username")?;
        let p = CString::new(password).map_err(|_| "invalid password")?;
        rc_client_begin_login_with_password(
            client,
            u.as_ptr(),
            p.as_ptr(),
            login_callback,
            &slot as *const LoginResultSlot as *mut c_void,
        );
        loop {
            if let Some(outcome) = slot.0.lock().unwrap().take() {
                if outcome.result == 0 {
                    if let Some(user) = get_user() {
                        crate::config::save_ra_credentials(&user.username, &user.token);
                    }
                    return Ok(());
                }
                return Err(outcome
                    .error
                    .unwrap_or_else(|| format!("login failed (rc_error {})", outcome.result)));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

pub fn logout() {
    let client = client_ptr();
    if !client.is_null() {
        unsafe { rc_client_logout(client) };
        crate::config::clear_ra_credentials();
        println!("[RA] logged out");
    }
}

#[allow(dead_code)]
pub struct RaUser {
    pub username: String,
    pub token: String,
    pub display_name: String,
    pub score: u32,
    pub score_softcore: u32,
    pub avatar_url: String,
    pub num_unread_messages: u32,
}

pub fn get_user() -> Option<RaUser> {
    let client = client_ptr();
    if client.is_null() {
        return None;
    }
    unsafe {
        let user = rc_client_get_user_info(client);
        if user.is_null() {
            return None;
        }
        let u = &*user;
        Some(RaUser {
            username: cstr_lossy(u.username),
            token: cstr_lossy(u.token),
            display_name: cstr_lossy(u.display_name),
            score: u.score,
            score_softcore: u.score_softcore,
            avatar_url: cstr_lossy(u.avatar_url),
            num_unread_messages: u.num_unread_messages,
        })
    }
}

pub struct RaGame {
    pub id: u32,
    pub title: String,
}

pub fn get_game() -> Option<RaGame> {
    let client = client_ptr();
    if client.is_null() {
        return None;
    }
    unsafe {
        let game = rc_client_get_game_info(client);
        if game.is_null() {
            return None;
        }
        let g = &*game;
        Some(RaGame {
            id: g.id,
            title: cstr_lossy(g.title),
        })
    }
}

pub fn game_loaded() -> bool {
    let client = client_ptr();
    if client.is_null() {
        return false;
    }
    unsafe { rc_client_is_game_loaded(client) != 0 }
}

static PENDING_HASH: Mutex<Option<String>> = Mutex::new(None);

pub fn compute_ra_hashes(path: &str, rom: &[u8]) -> Vec<String> {
    let mut hashes = Vec::new();
    if rom.is_empty() {
        return hashes;
    }
    let c_path = match CString::new(path) {
        Ok(p) => p,
        Err(_) => return hashes,
    };
    unsafe {
        let mut iter = [0u8; HASH_BUF_SIZE];
        rc_hash_initialize_iterator(
            iter.as_mut_ptr(),
            c_path.as_ptr(),
            rom.as_ptr(),
            rom.len(),
        );
        let mut hash_buf = [0u8; 33];
        for _ in 0..4 {
            let result = rc_hash_iterate(hash_buf.as_mut_ptr(), iter.as_mut_ptr());
            if result == 0 {
                break;
            }
            let hash = CStr::from_ptr(hash_buf.as_ptr() as *const c_char)
                .to_string_lossy()
                .into_owned();
            if hash.is_empty() {
                break;
            }
            println!("[RA] hash for {}: {}", path, hash);
            hashes.push(hash);
        }
        rc_hash_destroy_iterator(iter.as_mut_ptr());
    }

    if let Some(first) = hashes.first().cloned() {
        *PENDING_HASH.lock().unwrap() = Some(first);
    }
    hashes
}

pub fn take_pending_hash() -> Option<String> {
    PENDING_HASH.lock().unwrap().take()
}

pub fn load_game(hash: &str) -> Result<String, String> {
    let client = client_ptr();
    if client.is_null() {
        return Err("RetroAchievements client not initialized".to_string());
    }
    unsafe {
        let slot = LoadResultSlot(Mutex::new(None));
        let h = CString::new(hash).map_err(|_| "invalid hash")?;
        rc_client_begin_load_game(
            client,
            h.as_ptr(),
            load_game_callback,
            &slot as *const LoadResultSlot as *mut c_void,
        );
        loop {
            if let Some(outcome) = slot.0.lock().unwrap().take() {
                if outcome.result == 0 {
                    if let Some(game) = get_game() {
                        println!(
                            "[RA] game loaded: {} (id {}) — {} mode",
                            game.title,
                            game.id,
                            if hardcore_active() { "hardcore" } else { "softcore" }
                        );
                        return Ok(game.title);
                    }
                    return Ok("unknown game".to_string());
                }
                return Err(outcome
                    .error
                    .unwrap_or_else(|| format!("load failed (rc_error {})", outcome.result)));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

pub fn unload_game() {
    let client = client_ptr();
    if !client.is_null() && game_loaded() {
        unsafe { rc_client_unload_game(client) };
        println!("[RA] game unloaded");
    }
}

#[allow(dead_code)]
pub fn set_hardcore(enabled: bool) {
    let client = client_ptr();
    if !client.is_null() {
        unsafe { rc_client_set_hardcore_enabled(client, enabled as i32) };
        HARDCORE_ACTIVE.store(enabled, Ordering::Relaxed);
    }
}

pub fn do_frame(emu: &Emulator) {
    let client = client_ptr();
    if client.is_null() {
        return;
    }
    set_emu_thread_ptr(emu as *const Emulator as u64);
    unsafe { rc_client_do_frame(client) };
    clear_emu_thread_ptr();
}

pub fn idle() {
    let client = client_ptr();
    if !client.is_null() {
        unsafe { rc_client_idle(client) };
    }
}

pub fn take_reset_requested() -> bool {
    RESET_REQUESTED.swap(false, Ordering::Relaxed)
}

use crate::emulator::Emulator;
