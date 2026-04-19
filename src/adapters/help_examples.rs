pub fn for_humans() -> &'static str {
    "\x1b[1;33mExamples:\x1b[0m
  yx add \"fix the flaky test\"
  yx add \"upgrade auth library\" --under \"fix the flaky test\"
  yx list
  yx show \"fix the flaky test\"
  yx start \"fix the flaky test\"
  yx tag \"fix the flaky test\" bug
  yx field \"fix the flaky test\" priority <<< \"high\"
  yx list --tag bug
  yx done \"fix the flaky test\"

\x1b[1;33mDependencies:\x1b[0m
  Children block their parent. Put prerequisites \x1b[1munder\x1b[0m the goal:

  yx add \"deploy app\"
  yx add \"write tests\" --under \"deploy app\"
  yx add \"set up CI\" --under \"deploy app\"
  yx add \"fix linter\" --under \"set up CI\"

  deploy app
  ├── write tests      ← can do in parallel
  └── set up CI        ← with this one
      └── fix linter   ← but do this first"
}

pub fn for_agents() -> &'static str {
    "\x1b[1;33mExamples:\x1b[0m
  yx add \"fix the flaky test\" --under \"parent yak\"
  yx show \"fix the flaky test\"
  yx context \"fix the flaky test\" <<< $(cat <<'EOF'
The login test fails intermittently due to a race
condition in the session cleanup code.
EOF
)
  yx field \"fix the flaky test\" plan <<< \"Step 1: ...\"
  yx tag \"fix the flaky test\" bug
  yx list --tag bug --format json
  yx done \"fix the flaky test\"

\x1b[1;33mDependencies:\x1b[0m
  Children block their parent. Put prerequisites under the goal:

  yx add \"deploy app\"
  yx add \"write tests\" --under \"deploy app\"
  yx add \"set up CI\" --under \"deploy app\"
  yx add \"fix linter\" --under \"set up CI\"

  deploy app
  ├── write tests      ← can do in parallel
  └── set up CI        ← with this one
      └── fix linter   ← but do this first"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_examples_contain_start() {
        assert!(for_humans().contains("yx start"));
    }

    #[test]
    fn human_examples_do_not_contain_format_json() {
        assert!(!for_humans().contains("--format json"));
    }

    #[test]
    fn agent_examples_contain_format_json() {
        assert!(for_agents().contains("--format json"));
    }

    #[test]
    fn agent_examples_do_not_contain_start() {
        assert!(!for_agents().contains("yx start"));
    }
}
