use std::io::{stdout, Write};
use crossterm::event::{read, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::app::AppContext;
use crate::git;
use crate::system::{system, system_safe, system_stream};
use crate::ui::common::{with_terminal_pause};

pub fn git_push(_ctx: &mut AppContext) -> anyhow::Result<()> {
    with_terminal_pause(|| {
        println!("Fetching from remote...");
        let _ = system_stream("git fetch --prune");

        println!("\n\x1b[1;32mCurrent file status...\x1b[0m");
        let _ = system_stream("git -c color.status=always status -s");

        let current = git::get_current_branch()?;
        let tracking = git::get_tracking_branch().unwrap_or_default();

        println!("\n\x1b[1;36mLocal branch:\x1b[0m {}", current);
        if !tracking.is_empty() {
            println!("\x1b[1;35mRemote branch:\x1b[0m {}", tracking);
            println!("\n\x1b[1;32mCommits (context):\x1b[0m");
            let log_cmd = format!("git log --color --oneline --graph --decorate --abbrev-commit -n 15 {}~3..{}", tracking, current);
            if system_stream(&log_cmd).unwrap_or(1) != 0 {
                let _ = system_stream(&format!("git log --color --oneline --graph --decorate --abbrev-commit -n 15 {}..{}", tracking, current));
            }
        } else {
            println!("\x1b[1;33mNo tracking branch found.\x1b[0m");
            println!("\n\x1b[1;32mRecent commits:\x1b[0m");
            let _ = system_stream("git log --color --oneline --graph --decorate --abbrev-commit -n 10");
        }

        let mut targets = Vec::new();
        
        if let Ok(out) = system("git branch -r --format='%(refname:short)'") {
            for line in out.lines() {
                let line = line.trim();
                if line.is_empty() { continue; }
                
                if let Some(pos) = line.find('/') {
                    let branch_name = &line[pos+1..];
                    if !targets.contains(&branch_name.to_string()) {
                        targets.push(branch_name.to_string());
                    }
                }
            }
        }

        if !tracking.is_empty() {
            let tracking_short = tracking.split('/').last().unwrap_or("").to_string();
            if let Some(pos) = targets.iter().position(|x| x == &tracking_short) {
                targets.remove(pos);
            }
            targets.insert(0, tracking_short);
        } else if let Some(pos) = targets.iter().position(|x| x == &current) {
            targets.remove(pos);
            targets.insert(0, current.clone());
        }

        let target = interactive_push_selector(&targets)?;
        if target.is_empty() {
            println!("Push canceled.");
            return Ok(())
        }

        let remote = if tracking.contains('/') {
            tracking.split('/').next().unwrap_or("origin").to_string()
        } else {
            "origin".to_string()
        };

        println!("\nPushing to {}/{}:..", remote, target);
        let cmd = format!("git push {} {}:{}", remote, current, target);
        let (out, code) = system_safe(&cmd);
        println!("{}", out);
        if code != 0 {
            println!("\x1b[1;31mPush failed.\x1b[0m");
        } else {
            println!("\x1b[1;32mPush successful.\x1b[0m");
        }
        
        println!("\nPress Enter to return...");
        let mut tmp = String::new();
        let _ = std::io::stdin().read_line(&mut tmp);
        Ok(())
    })
}

fn interactive_push_selector(items: &[String]) -> anyhow::Result<String> {
    if items.is_empty() {
        print!("Input remote branch name: ");
        stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        return Ok(input.trim().to_string());
    }

    let mut selected_idx = 0;
    let mut input = items[0].clone();
    
    enable_raw_mode()?;
    
    let res = loop {
        print!("\r\x1b[KPush to remote branch: \x1b[1;36m{}\x1b[0m", input);
        print!("\n\r\x1b[K(Suggestions: Use Up/Down arrows to select)");
        for (i, item) in items.iter().enumerate() {
            if i == selected_idx {
                print!("\n\r\x1b[K > \x1b[1;32m{}\x1b[0m", item);
            } else {
                print!("\n\r\x1b[K   {}", item);
            }
        }
        print!("\x1b[{}A", items.len() + 1);
        print!("\r\x1b[{}C", 24 + input.len());
        stdout().flush()?;

        match read()? {
            Event::Key(event) => {
                match event.code {
                    KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                        disable_raw_mode()?;
                        return Err(anyhow::anyhow!("Interrupted by user"));
                    }
                    KeyCode::Enter => {
                        break Ok(input.trim().to_string());
                    }
                    KeyCode::Esc => {
                        break Ok(String::new());
                    }
                    KeyCode::Up => {
                        selected_idx = if selected_idx == 0 { items.len() - 1 } else { selected_idx - 1 };
                        input = items[selected_idx].clone();
                    }
                    KeyCode::Down => {
                        selected_idx = (selected_idx + 1) % items.len();
                        input = items[selected_idx].clone();
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    };

    print!("\r\x1b[K\x1b[{}B\n", items.len() + 1);
    stdout().flush()?;
    disable_raw_mode()?;
    res
}
