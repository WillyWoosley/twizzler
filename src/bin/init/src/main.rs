//#![no_std]
#![feature(lang_items)]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(thread_local)]
#![feature(duration_constants)]
//#![no_main]

/*
#[no_mangle]
pub extern "C" fn std_runtime_starta() {
    twizzler_abi::syscall::sys_kernel_console_write(
        b"hello world\n",
        twizzler_abi::syscall::KernelConsoleWriteFlags::empty(),
    );
    loop {}
}
*/

/*
#[panic_handler]
pub fn __panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
*/

#[thread_local]
static mut FOO: u32 = 42;
#[thread_local]
static mut BAR: u32 = 0;
#[allow(named_asm_labels)]

static BAZ: AtomicU64 = AtomicU64::new(0);

fn test_thread_sync() {
    let _j = std::thread::spawn(|| {
        let reference = ThreadSyncReference::Virtual(&BAZ as *const AtomicU64);
        let value = 0;
        let wait = ThreadSync::new_sleep(ThreadSyncSleep::new(
            reference,
            value,
            twizzler_abi::syscall::ThreadSyncOp::Equal,
            ThreadSyncFlags::empty(),
        ));

        loop {
            println!("{:?} going to sleep", std::thread::current().id());
            let res = sys_thread_sync(&mut [wait], None);
            println!("woke up: {:?} {:?}", res, wait.get_result());
        }
    });

    let reference = ThreadSyncReference::Virtual(&BAZ as *const AtomicU64);
    let wake = ThreadSync::new_wake(ThreadSyncWake::new(reference, 1));
    let mut c = 0u64;
    loop {
        println!("{:?} waking up {}", std::thread::current().id(), c);
        c += 1;
        let res = sys_thread_sync(&mut [wake], None);
        for _i in 0u64..40000u64 {}
        println!("done {:?}", res);
    }
}

fn test_thread_sync_timeout() {
    let _j = std::thread::spawn(|| {
        let reference = ThreadSyncReference::Virtual(&BAZ as *const AtomicU64);
        let value = 0;
        let wait = ThreadSync::new_sleep(ThreadSyncSleep::new(
            reference,
            value,
            twizzler_abi::syscall::ThreadSyncOp::Equal,
            ThreadSyncFlags::empty(),
        ));

        let mut c = 0u64;
        loop {
            println!("{:?} going to sleep {}", std::thread::current().id(), c);
            let res = sys_thread_sync(&mut [wait], Some(Duration::MILLISECOND * 1000));
            println!("woke up: {:?} {:?}", res, wait.get_result());
            c += 1;
        }
    });

    let reference = ThreadSyncReference::Virtual(&BAZ as *const AtomicU64);
    let _wake = ThreadSync::new_wake(ThreadSyncWake::new(reference, 1));
    let mut _c = 0u64;
    loop {

        // println!("{:?} waking up {}", std::thread::current().id(), c);
        // c += 1;
        // let res = sys_thread_sync(&mut [wake], None);
        // for i in 0u64..40000u64 {}
        // println!("done {:?}", res);
    }
}

struct Foo {
    x: u64,
}

fn test_mutex() {
    let mutex: Arc<Mutex<Foo>> = Arc::new(Mutex::new(Foo { x: 0 }));
    let mutex2 = mutex.clone();
    std::thread::spawn(move || {
        let mut c = 0u64;
        loop {
            let mut data = mutex.lock().unwrap();
            data.x += 1;
            let v = data.x;
            c += 1;
            if c % 1000000 == 0 {
                println!("w {}", data.x);
            }
            assert_eq!(v, data.x);
        }
    });

    let mut c = 0u64;
    loop {
        let mut data = mutex2.lock().unwrap();
        data.x += 1;
        c += 1;
        let v = data.x;
        // for i in 0..1000 {}
        assert_eq!(v, data.x);
        if c % 1000000 == 0 {
            println!("a {}", data.x);
        }
        assert_eq!(v, data.x);
    }
}

fn get_user_input() {
    println!("enter some text:");
    let mut s = String::new();
    std::io::stdin()
        .read_line(&mut s)
        .expect("Did not enter a correct string");
    println!("you typed: {}", s);
}

fn list_subobjs(level: usize, id: ObjID) {
    let mut n = 0;
    loop {
        let res = sys_kaction(
            KactionCmd::Generic(KactionGenericCmd::GetSubObject(
                SubObjectType::Info.into(),
                n,
            )),
            Some(id),
            0,
            KactionFlags::empty(),
        );
        if res.is_err() {
            break;
        } else if let KactionValue::ObjID(id) = res.unwrap() {
            println!("  sub {:indent$}info {}: {}", "", n, id, indent = level);
        }
        n += 1;
    }

    let mut n = 0;
    loop {
        let res = sys_kaction(
            KactionCmd::Generic(KactionGenericCmd::GetSubObject(
                SubObjectType::Mmio.into(),
                n,
            )),
            Some(id),
            0,
            KactionFlags::empty(),
        );
        if res.is_err() {
            break;
        } else if let KactionValue::ObjID(id) = res.unwrap() {
            println!("  sub {:indent$}mmio {}: {}", "", n, id, indent = level);
        }
        n += 1;
    }
}

fn enumerate_children(level: usize, id: ObjID) {
    let mut n = 0;
    loop {
        let res = sys_kaction(
            KactionCmd::Generic(KactionGenericCmd::GetChild(n)),
            Some(id),
            0,
            KactionFlags::empty(),
        );
        if res.is_err() {
            break;
        } else if let KactionValue::ObjID(id) = res.unwrap() {
            println!("{:indent$}{}: {}", "", n, id, indent = level);
            list_subobjs(level, id);
            enumerate_children(level + 4, id);
        }
        n = n + 1;
    }
}

fn test_kaction() {
    let res = sys_kaction(
        KactionCmd::Generic(KactionGenericCmd::GetKsoRoot),
        None,
        0,
        KactionFlags::empty(),
    );
    println!("{:?}", res);
    let id = match res.unwrap() {
        KactionValue::U64(_) => todo!(),
        KactionValue::ObjID(id) => id,
    };

    enumerate_children(0, id);
}

fn exec(name: &str, id: ObjID, argid: ObjID) {
    let env: Vec<String> = std::env::vars()
        .map(|(n, v)| format!("{}={}", n, v))
        .collect();
    let env_ref: Vec<&[u8]> = env.iter().map(|x| x.as_str().as_bytes()).collect();
    let mut args = vec![name.as_bytes()];
    let argstr = format!("{}", argid.as_u128());
    args.push(argstr.as_bytes());
    let _elf = twizzler_abi::load_elf::spawn_new_executable(id, &args, &env_ref);
    //println!("ELF: {:?}", elf);
}

fn find_init_name(name: &str) -> Option<ObjID> {
    let init_info = twizzler_abi::aux::get_kernel_init_info();
    for n in init_info.names() {
        if n.name() == name {
            return Some(n.id());
        }
    }
    None
}

fn main() {
    println!("[init] starting userspace");
    let _foo = unsafe { FOO + BAR };
    println!("Hello, World {}", unsafe { FOO + BAR });

    let create = ObjectCreate::new(
        BackingType::Normal,
        LifetimeType::Volatile,
        None,
        ObjectCreateFlags::empty(),
    );
    let netid = twizzler_abi::syscall::sys_object_create(create, &[], &[]).unwrap();
    /*
    if let Some(id) = find_init_name("devmgr") {
        exec("devmgr", id, ObjID::new(0));
    } else {
        eprintln!("[init] failed to start devmgr");
    }
    */
    if let Some(id) = find_init_name("netmgr") {
        exec("netmgr", id, netid);
    } else {
        eprintln!("[init] failed to start netmgr");
    }
    loop {
        let reply = rprompt::prompt_reply_stdout("> ").unwrap();
        println!("got: <{}>", reply);
        let cmd: Vec<&str> = reply.split(" ").collect();
        if cmd.len() == 2 && cmd[0] == "run" {
            if let Some(id) = find_init_name(cmd[1]) {
                if cmd[1] == "nettest" {
                    exec(cmd[1], id, netid);
                } else {
                    exec(cmd[1], id, ObjID::new(0));
                }
            } else {
                eprintln!("[init] failed to start {}", cmd[1]);
            }
        }

        //  get_user_input();
    }
    if false {
        test_kaction();
        get_user_input();
        test_thread_sync_timeout();
        test_mutex();
        test_thread_sync();
    }

    loop {}
}

/*
#[naked]
#[no_mangle]
extern "C" fn _start() -> ! {
    unsafe { asm!("call std_runtime_start", options(noreturn)) }
}
*/

use std::{
    sync::{atomic::AtomicU64, Arc, Mutex},
    time::Duration,
};

use twizzler_abi::{
    device::SubObjectType,
    kso::{KactionCmd, KactionFlags, KactionGenericCmd, KactionValue},
    object::ObjID,
    syscall::{
        sys_kaction, sys_thread_sync, BackingType, LifetimeType, ObjectCreate, ObjectCreateFlags,
        ThreadSync, ThreadSyncFlags, ThreadSyncReference, ThreadSyncSleep, ThreadSyncWake,
    },
};
