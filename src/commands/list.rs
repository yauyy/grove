use anyhow::Result;
use console::Style;

use crate::config;

pub fn run() -> Result<()> {
    let pf = config::load_projects()?;

    if pf.projects.is_empty() {
        println!("No projects registered. Use `grove add <path>` to add one.");
        return Ok(());
    }

    let bold = Style::new().bold();
    let dim = Style::new().dim();

    // Sort groups by order
    let mut groups = pf.groups.clone();
    groups.sort_by_key(|g| g.order);

    // Print grouped projects
    for group in &groups {
        let group_projects: Vec<_> = pf
            .projects
            .iter()
            .filter(|p| p.group == group.name)
            .collect();

        if group_projects.is_empty() {
            continue;
        }

        println!("{}", bold.apply_to(&group.name));
        let mut sorted_projects = group_projects;
        sorted_projects.sort_by_key(|p| p.order);

        for project in sorted_projects {
            print_project(project, &dim);
        }
        println!();
    }

    // Print ungrouped projects
    let ungrouped: Vec<_> = pf
        .projects
        .iter()
        .filter(|p| p.group.is_empty())
        .collect();

    if !ungrouped.is_empty() {
        println!("{}", bold.apply_to("Ungrouped"));
        let mut sorted_ungrouped = ungrouped;
        sorted_ungrouped.sort_by_key(|p| p.order);

        for project in sorted_ungrouped {
            print_project(project, &dim);
        }
        println!();
    }

    Ok(())
}

fn print_project(project: &config::Project, dim: &Style) {
    let mut branch_info = project.branches.main.clone();

    let mut env_parts: Vec<String> = Vec::new();
    if let Some(ref t) = project.branches.test {
        env_parts.push(format!("test:{}", t));
    }
    if let Some(ref s) = project.branches.staging {
        env_parts.push(format!("stg:{}", s));
    }
    if let Some(ref p) = project.branches.prod {
        env_parts.push(format!("prod:{}", p));
    }
    if !env_parts.is_empty() {
        branch_info = format!("{} [{}]", branch_info, env_parts.join(", "));
    }

    println!(
        "  {} {} {}",
        project.name,
        dim.apply_to(&project.path),
        dim.apply_to(format!("({})", branch_info))
    );
}
