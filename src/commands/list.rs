use anyhow::Result;
use console::Style;

use crate::config;
use crate::i18n::t;

pub fn run() -> Result<()> {
    let pf = config::load_projects()?;

    if pf.projects.is_empty() {
        println!("{}", t("no_projects"));
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
    let ungrouped: Vec<_> = pf.projects.iter().filter(|p| p.group.is_empty()).collect();

    if !ungrouped.is_empty() {
        println!("{}", bold.apply_to(t("ungrouped")));
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
    println!(
        "{}",
        format_project_line(project, dim, &Style::new().cyan().bold())
    );
}

fn format_project_line(project: &config::Project, dim: &Style, tag_style: &Style) -> String {
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

    let tags = format_tags(&project.tags, tag_style);
    let name = if tags.is_empty() {
        project.name.clone()
    } else {
        format!("{} {}", project.name, tags)
    };

    format!(
        "  {} {} {}",
        name,
        dim.apply_to(&project.path),
        dim.apply_to(format!("({})", branch_info))
    )
}

fn format_tags(tags: &[String], tag_style: &Style) -> String {
    tags.iter()
        .map(|tag| tag_style.apply_to(format!("[{}]", tag)).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with_tags(tags: Vec<&str>) -> config::Project {
        config::Project {
            name: "api".to_string(),
            path: "/tmp/api".to_string(),
            group: String::new(),
            order: 0,
            tags: tags.into_iter().map(str::to_string).collect(),
            agents_md: None,
            branches: config::BranchConfig {
                main: "main".to_string(),
                test: None,
                staging: None,
                prod: None,
            },
        }
    }

    #[test]
    fn test_format_project_line_shows_tags() {
        let line =
            format_project_line(&project_with_tags(vec!["go"]), &Style::new(), &Style::new());

        assert!(line.contains("api [go]"));
    }

    #[test]
    fn test_format_project_line_omits_empty_tags() {
        let line = format_project_line(&project_with_tags(vec![]), &Style::new(), &Style::new());

        assert!(line.contains("api /tmp/api"));
        assert!(!line.contains("[]"));
    }
}
