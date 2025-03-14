use std::env;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use serde_json::{Value, from_str};
use regex::Regex;
use std::path::Path;

fn main() {
    println!("当前版本：v1
支持系统：windows, linux

软件使用方式
方式一：windows
  1. 将软件拖入到需要解压的文件夹
  2. 输入仓库路径和密码
  3. 解压完成

方式二：windows, linux
  1. restic_decompress.exe <仓库路径>
  2. 输入仓库密码
  3. 解压完成");
    let restic_result = check_restic_path();
    if restic_result == 0 {
        println!("你的电脑未安装 restic，程序当前目录现在没有检测到文件 restic.exe");
        println!("请按任意键退出...");
        io::stdin().read_line(&mut String::new()).unwrap();
        return;
    }
    let restic_exe_path = match restic_result {
        1 => {
            println!("使用系统PATH中的restic");
            "restic".to_string()
        },
        2 => {
            let exe_path = env::current_exe().expect("获取程序路径失败");
            let script_dir = exe_path.parent().expect("无法获取程序所在目录");
            let restic_exe_path = script_dir.join("restic.exe");
            println!("使用当前目录的restic.exe：{}", restic_exe_path.display());
            restic_exe_path.to_string_lossy().into_owned()
        }
        _ => return, // 如果 restic_result 不是 1 或 2，则直接返回，因为在 check_restic_path 函数中，0 代表未找到 restic
    };

    // 获取目标路径（支持拖放和命令行参数）
    let args: Vec<String> = env::args().collect();
    let target_path = if args.len() > 1 {
        args[1].trim().to_string()
    } else {
        println!("请输入仓库路径：");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("读取路径失败");
        input.trim().to_string()
    };

    // 获取用户输入密码
    println!("请输入仓库密码：");
    let mut passwd = String::new();
    io::stdin().read_line(&mut passwd).expect("读取密码失败");
    let passwd = passwd.trim_end();

    // 获取当前可执行文件所在目录
    let exe_path = env::current_exe().expect("获取程序路径失败");
    // let script_path = exe_path.parent()
    //     .expect("无法获取程序所在目录")
    //     .to_str()
    //     .expect("路径转换失败");

    // 获取当前可执行文件所在目录作为输出路径
    let output_path = exe_path.parent()
        .expect("无法获取程序所在目录")
        .to_str()
        .expect("路径转换失败");

    // 调用恢复函数
    let restore_result = restic_restore(&restic_exe_path, &target_path, output_path, passwd, "latest");
    
    match restore_result {
        Ok(success_msg) => println!("{}", success_msg),
        Err(err_msg) => {
            eprintln!("{}", err_msg);
            // 如果是密码错误直接返回，不执行后续输出
            if err_msg.contains("密码错误") {
                println!("密码错误，点击任意键退出...");
                io::stdin().read_line(&mut String::new()).unwrap();
                return;
            }
            if err_msg.contains("仓库路径错误") {
                println!("仓库路径错误，点击任意键退出...");
                io::stdin().read_line(&mut String::new()).unwrap();
                return;
            }
        }
    }

    // 只有非密码错误的情况才会执行以下输出
    println!("解压完成，请按任意键退出...");
    io::stdin().read_line(&mut String::new()).unwrap();
}

/// 恢复指定快照到指定目录
///
/// # 参数
/// - `restic_exe_path`: restic 可执行文件路径（需包含文件名）
/// - `restic_path`: restic 仓库路径，对应命令位置 restic -r <restic_path>
/// - `output_path`: 恢复文件的目标输出路径
/// - `passwd`: 仓库访问密码
/// - `snapshot_id`: 快照标识，支持 "latest" 表示最新快照，或指定完整快照ID
///
/// # 返回值
/// - `Ok(String)`: 包含成功恢复路径的字符串（格式："恢复成功到: {output_path}"）
/// - `Err(String)`: 包含错误描述的字符串，可能原因包括：
///   - 快照未找到
///   - JSON解析失败
///   - 密码验证失败
///   - 子进程执行错误
///
/// # 示例
/// ```
/// let result = restic_restore("restic", "/backup", "./restored", "mypass", "latest");
/// match result {
///     Ok(msg) => println!("{}", msg),
///     Err(e) => eprintln!("错误: {}", e),
/// }
/// ```
fn restic_restore(restic_exe_path: &str, restic_path: &str, output_path: &str, passwd: &str, snapshot_id: &str) -> Result<String, String> {
    // 获取快照列表
    // let output = Command::new("restic")
    //     .args(["-r", restic_path, "snapshots", "--json"])
    //     .stdin(std::process::Stdio::piped())
    //     .stdout(std::process::Stdio::piped())
    //     .stderr(std::process::Stdio::piped())
    //     .spawn()
    //     .map_err(|e| e.to_string())?
    //     .wait_with_output()
    //     .map_err(|e| e.to_string())?;

    // 输入密码
    let mut child = Command::new(restic_exe_path)
        .args(["-r", restic_path, "snapshots", "--json"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    
    std::io::Write::write_all(&mut child.stdin.as_mut().unwrap(), passwd.as_bytes()).map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    
    // 新增密码错误检测
    let stderr_output = String::from_utf8_lossy(&output.stderr);
    if stderr_output.contains("wrong password or no key found") {
        return Err("密码错误".to_string());
    }
    // 新增仓库路径检测
    if stderr_output.contains("repository does not exist") {
        return Err("仓库路径错误".to_string());
    }

    // 解析JSON
    let output_str = String::from_utf8_lossy(&output.stdout);
    let re = Regex::new(r"\[.*").unwrap();
    let json_str = re.captures(&output_str)
        .ok_or("未在快照输出结果中找到 JSON 数据")?
        .get(0).unwrap().as_str();
    
    let data: Value = from_str(json_str).map_err(|e| e.to_string())?;
    let mut snapshot_path = None;

    if let Some(snapshots) = data.as_array() {
        for snap in snapshots {
            let sid = snap["short_id"].as_str().unwrap_or("");
            if snapshot_id == "latest" || sid == snapshot_id {
                if let Some(paths) = snap["paths"].as_array() {
                    if let Some(path) = paths[0].as_str() {
                        let processed_path = path.replace("\\\\", "/")
                            .replace("\\", "/")
                            .replacen(":", "", 1);
                        snapshot_path = PathBuf::from(processed_path)
                            .parent()
                            .map(|p| p.to_string_lossy().into_owned());
                        break;
                    }
                }
            }
        }
    }

    let snapshot_path = snapshot_path.ok_or(format!("未找到 snapshot_id 为 {} 的快照", snapshot_id))?;

    // 执行恢复命令
    let mut restore_cmd = Command::new(restic_exe_path)
        .args(["-r", restic_path, "restore", &format!("{}:{}", snapshot_id, snapshot_path), "--target", output_path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    std::io::Write::write_all(&mut restore_cmd.stdin.as_mut().unwrap(), passwd.as_bytes()).map_err(|e| e.to_string())?;
    let restore_output = restore_cmd.wait_with_output().map_err(|e| e.to_string())?;

    if restore_output.status.success() {
        Ok(String::from_utf8_lossy(&restore_output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&restore_output.stderr).into_owned())
    }
}

/// 检查系统中 restic 的可用性
/// 
/// 本函数会按以下顺序检查 restic 是否存在：
/// 1. 检查系统 PATH 环境变量中是否存在 restic 命令
/// 2. 检查程序所在目录是否存在 restic.exe 文件
/// 
/// # 返回值
/// - 返回 1 表示在系统 PATH 中找到 restic
/// - 返回 2 表示在当前程序目录找到 restic.exe
/// - 返回 0 表示未找到任何可用的 restic
/// 
/// # 注意
/// 当返回 0 时，主程序会提示用户并退出
fn check_restic_path() -> i8 {
    // 1. 检查系统 PATH 中是否有 restic 命令
    let output = Command::new("restic")
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let re = Regex::new(r"restic is a backup program").unwrap();
            if re.is_match(&stdout) {
                return 1;
            }
        }
    }

    // 2. 查找当前脚本所在目录下是否有 restic.exe
    let exe_path = env::current_exe().expect("获取程序路径失败");
    let script_dir = exe_path.parent().expect("无法获取程序所在目录");
    let restic_exe_path = script_dir.join("restic.exe");
    if Path::new(&restic_exe_path).exists() {
        return 2;
    }

    // 3. 如果以上两种方法都没找到，返回 0
    0
}