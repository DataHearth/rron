latest-tag := `git describe --tags --abbrev=0`

publish version: (bump version)
  git-chglog --next-tag {{version}} --output CHANGELOG.md
  git add CHANGELOG.md Cargo.toml flake.nix && git commit -m "chore(changelog): release {{version}}"
  git tag -a {{version}} -m "{{version}}"
  git push --follow-tags

bump version:
  sed -i 's/{{replace(latest-tag, "v", "")}}/{{replace(version, "v", "")}}/' Cargo.toml flake.nix
