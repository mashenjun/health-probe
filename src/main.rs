use actix_web::http::header::ContentType;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::{arg, command, Parser};
use libc::{self};
use nix::errno::Errno;
use nix::sys::socket::{
    connect, setsockopt as nix_setsockopt, socket, sockopt::Linger, AddressFamily, SockFlag,
    SockType, SockaddrIn,
};
use nix::unistd::close;
use std::env;
use std::mem::size_of;
use std::str::FromStr;
use std::time::Duration;

#[allow(unused_macros)]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// port to listen on
    #[arg(short, long, default_value_t = String::from("80") )]
    listen_port: String,

    /// address to probe
    #[arg(short, long, default_value_t = String::from("0.0.0.0:8085"))]
    probe_addr: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // read args variables.
    let args = Args::parse();
    let mut listen_port = args.listen_port;
    let mut probe_addr = SockaddrIn::from_str(args.probe_addr.as_str()).unwrap();
    // env variables can override args variables.
    if let Ok(port) = env::var("LISTEN_PORT") {
        listen_port = port.to_string();
    }
    if let Ok(ip_port) = env::var("PROBE_ADDR") {
        probe_addr = SockaddrIn::from_str(ip_port.as_str()).unwrap();
    }
    println!(
        "listen on: {:?}, probe target is: {:?}",
        listen_port,
        probe_addr.to_string()
    );
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(ProbeApp {
                probe_addr: probe_addr,
            }))
            .route("/health-probe", web::get().to(health_probe))
    })
    .keep_alive(Duration::from_secs(60))
    .bind(format!("0.0.0.0:{listen_port}"))?
    .run()
    .await
}

struct ProbeApp {
    probe_addr: SockaddrIn,
}
// todo(shenjun): make check_status async
fn check_status(addr: &SockaddrIn) -> i32 {
    let linger_opt = libc::linger {
        l_onoff: 1,
        l_linger: 0,
    };
    if let Ok(sk) = socket(
        AddressFamily::Inet,
        SockType::Stream,
        SockFlag::SOCK_NONBLOCK,
        None,
    ) {
        // set socket options
        // TODO(shenjun): add reference link to haproxy source code.
        if syscall!(setsockopt(
            sk,
            libc::SOL_TCP,
            libc::TCP_NODELAY,
            &1 as *const libc::c_int as *const libc::c_void,
            size_of::<libc::c_int>() as libc::socklen_t
        ))
        .is_err()
        {
            println!("setscokopt to enable no delay failed");
            return 500;
        }
        if syscall!(setsockopt(
            sk,
            libc::SOL_TCP,
            libc::TCP_QUICKACK,
            &0 as *const libc::c_int as *const libc::c_void,
            size_of::<libc::c_int>() as libc::socklen_t
        ))
        .is_err()
        {
            println!("setscokopt to dsiable quick ack failed");
            return 500;
        }
        if let Err(e) = connect(sk, addr) {
            if e != Errno::EINPROGRESS {
                println!("try connect {} failed, errno: {}", addr.to_string(), e);
            }
        }
        let mut pollfd = libc::pollfd {
            fd: sk,
            events: libc::POLLIN | libc::POLLOUT | libc::POLLRDHUP,
            revents: 0,
        };
        let poll_res = syscall!(poll(&mut pollfd, 1, 1000));
        if let Err(e) = nix_setsockopt(sk, Linger, &linger_opt) {
            println!("setscoketopt to update linger failed, err: {}", e);
        }
        let _ = close(sk);
        match poll_res {
            Ok(_) => {
                if pollfd.revents == libc::POLLOUT {
                    println!("target is alive");
                    return 200;
                }
                println!("pollfd got revents: {:b}", pollfd.revents);
                return 410;
            }
            Err(e) => {
                println!("poll socket failed, err: {}", e);
                return 500;
            }
        }
    } else {
        println!("create socket failed");
        return 500;
    }
}

async fn health_probe(data: web::Data<ProbeApp>) -> impl Responder {
    // check if target port is ready to connect
    let status = check_status(&data.probe_addr);
    if status == 200 {
        return HttpResponse::Ok()
            .content_type(ContentType::json())
            .insert_header(("X-Probe-Addr", data.probe_addr.to_string()))
            .body("{\"status\":\"ok\"}");
    }
    if status == 410 {
        return HttpResponse::Gone()
            .content_type(ContentType::json())
            .insert_header(("X-Probe-Addr", data.probe_addr.to_string()))
            .body("{\"status\":\"gone\"}");
    }
    return HttpResponse::InternalServerError()
        .content_type(ContentType::json())
        .insert_header(("X-Probe-Addr", data.probe_addr.to_string()))
        .body("{\"status\":\"probe health failed\"}");
}
