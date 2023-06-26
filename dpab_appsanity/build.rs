use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
const DISTP_REPOSITORY: &str = "ssh://git@github.comcast.com/distp/distro-protos.git";
const DISTP_COMMIT: &str = "ab04fbe23c67e9a43270e9d6893216fa883b2c55";
const DISTP_BRANCH: &str = "main";
const OTTX_PROTOBUF_REPO: &str = "ssh://git@github.comcast.com/ottx/ottx-protobuf.git";
const OTTX_PROTOBUF_TAG: &str = "1.147.0";
const OTTX_PROTOBUF_BRANCH: &str = "master";
const GOOGLE_APIS_REPO: &str = "https://github.com/googleapis/googleapis.git";
const GOOGLE_APIS_BRANCH: &str = "master";
const GOOGLE_APIS_COMMIT: &str = "969b95cfbac45b238be19afd7d9a4adf1412d748";
const BUF_REPO: &str = "https://github.com/bufbuild/protoc-gen-validate.git";
const BUF_BRANCH: &str = "main";
const BUF_COMMIT: &str = "dcefbbaa4a4810564eb6289aed0b23b38a57170c";

/*
Finally, tensorflow is useful!!!
 https://github.com/tensorflow/rust/blob/master/tensorflow-sys/build.rs
 */
macro_rules! get(($name:expr) => (ok!(env::var($name))));
macro_rules! ok(($expression:expr) => ($expression.unwrap()));
macro_rules! log {
    ($fmt:expr) => (println!(concat!("dpab_appsanity/build.rs:{}: ", $fmt), line!()));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("dpab_appsanity/build.rs:{}: ", $fmt),
    line!(), $($arg)*));
}

fn run<F>(name: &str, mut configure: F)
where
    F: FnMut(&mut Command) -> &mut Command,
{
    let mut command = Command::new(name);
    let configured = configure(&mut command);
    log!("Executing {:?}", configured);
    if !ok!(configured.status()).success() {
        panic!("failed to execute {:?}", configured);
    }
    log!("Command {:?} finished successfully", configured);
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let distp_source =
        PathBuf::from(&get!("CARGO_MANIFEST_DIR")).join(format!("target/source-distp-protos"));
    if !Path::new(&distp_source.join(".git")).exists() {
        run("git", |command| {
            command
                .arg("clone")
                .arg(format!("--branch={}", DISTP_BRANCH))
                .arg("--recursive")
                .arg(DISTP_REPOSITORY)
                .arg(&distp_source)
        });
        run("git", |command| {
            command
                .arg("-C")
                .arg(&distp_source)
                .arg("checkout")
                .arg(DISTP_COMMIT)
        });
    }

    let ottx_protobuf_source = PathBuf::from(&get!("CARGO_MANIFEST_DIR"))
        .join(format!("target/source-ottx-protobuf-{}", OTTX_PROTOBUF_TAG));
    if !Path::new(&ottx_protobuf_source.join(".git")).exists() {
        run("git", |command| {
            command
                .arg("clone")
                .arg(format!("--branch={}", OTTX_PROTOBUF_BRANCH))
                .arg("--recursive")
                .arg(OTTX_PROTOBUF_REPO)
                .arg(&ottx_protobuf_source)
        });
        run("git", |command| {
            command
                .arg("-C")
                .arg(&ottx_protobuf_source)
                .arg("checkout")
                .arg(OTTX_PROTOBUF_TAG)
        });
    }

    let google_apis_source = PathBuf::from(&get!("CARGO_MANIFEST_DIR"))
        .join(format!("target/source-google-apis-{}", GOOGLE_APIS_BRANCH));
    if !Path::new(&google_apis_source.join(".git")).exists() {
        run("git", |command| {
            command
                .arg("clone")
                .arg("--recursive")
                .arg(GOOGLE_APIS_REPO)
                .arg(&google_apis_source)
        });
        run("git", |command| {
            command
                .arg("-C")
                .arg(&google_apis_source)
                .arg("checkout")
                .arg(GOOGLE_APIS_COMMIT)
        });
    }

    let buf_source = PathBuf::from(&get!("CARGO_MANIFEST_DIR"))
        .join(format!("target/source-buf-{}", BUF_BRANCH));
    if !Path::new(&buf_source.join(".git")).exists() {
        run("git", |command| {
            command
                .arg("clone")
                .arg(format!("--branch={}", BUF_BRANCH))
                .arg("--recursive")
                .arg(BUF_REPO)
                .arg(&buf_source)
        });
        run("git", |command| {
            command
                .arg("-C")
                .arg(&buf_source)
                .arg("checkout")
                .arg(BUF_COMMIT)
        });
    }
    /*
    Do some "layering" (using that fancy word to paper over the fact that this is a hack)
    */
    run("sh", |command| {
        command.arg("-c").arg(format!(
            "cp -r {} {}",
            &google_apis_source.join("google").to_string_lossy(),
            distp_source
                .join("distp/gateway/secure_storage/v1/")
                .to_string_lossy()
        ))
    });

    run("sh", |command| {
        command.arg("-c").arg(format!(
            "cp -r {} {}",
            &buf_source.join("validate").to_string_lossy(),
            distp_source
                .join("distp/gateway/secure_storage/v1/")
                .to_string_lossy()
        ))
    });

    run("sh", |command| {
        command.arg("-c").arg(format!(
            "cp -r {} {}",
            &google_apis_source.join("google").to_string_lossy(),
            distp_source
                .join("distp/gateway/catalog/v1/")
                .to_string_lossy()
        ))
    });

    run("sh", |command| {
        command.arg("-c").arg(format!(
            "cp -r {} {}",
            &buf_source.join("validate").to_string_lossy(),
            distp_source
                .join("distp/gateway/catalog/v1/")
                .to_string_lossy()
        ))
    });

    tonic_build::compile_protos(
        ottx_protobuf_source
            .join(format!("src/main/resources/{}", "permission_service.proto"))
            .as_os_str(),
    )?;
    tonic_build::compile_protos(
        ottx_protobuf_source
            .join(format!("src/main/resources/{}", "ad_platform.proto"))
            .as_os_str(),
    )?;
    tonic_build::compile_protos(
        ottx_protobuf_source
            .join(format!("src/main/resources/{}", "resapi.proto"))
            .as_os_str(),
    )?;
    tonic_build::configure()
        .build_server(false)
        .compile(
            &[distp_source
                .join(format!(
                    "distp/gateway/secure_storage/v1/{}",
                    "secure_storage.proto"
                ))
                .as_os_str()],
            &[distp_source
                .join("distp/gateway/secure_storage/v1")
                .as_os_str()],
        )
        .unwrap();

    tonic_build::configure()
        .build_server(false)
        .compile(
            &[distp_source
                .join(format!("distp/gateway/catalog/v1/{}", "service.proto"))
                .as_os_str()],
            &[distp_source.join("distp/gateway/catalog/v1").as_os_str()],
        )
        .unwrap();
    Ok(())
}
