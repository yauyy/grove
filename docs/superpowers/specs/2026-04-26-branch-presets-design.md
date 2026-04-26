# Grove 分支预设与项目分支别名设计

## 背景

Grove 当前已经通过 `[projects.branches]` 支持项目级分支配置，字段主要是 `main`、`test`、`staging`、`prod`。现有批量 Git 命令会使用这些配置，例如 `gmerge` 通过环境分支配置执行合并，`gpush` 当前会推送 `workspaces.toml` 中记录的工作区分支。

这次新增能力的目标是支持更灵活的多项目分支工作流：

- `gmerge` 继续展示类似当前 `test`、`staging`、`prod` 的选项，但这些选项可以在全局配置中自定义。
- 每个项目可以把同一个全局选项映射到不同的真实 Git 分支名。
- 每个项目可以配置命令输入别名，让用户传入熟悉的分支名时，Grove 能按项目配置解析到真实分支。
- 新增 `gswitch`，用于在当前工作区中批量切换所有项目到解析后的目标分支。
- 新增 `gcreate`，用于在当前工作区中为所有项目创建并切换到新分支，新分支基于每个项目配置的最新主分支创建。
- 优化 `gpush`，支持推送指定的预设、项目别名、逻辑分支或真实分支。
- 创建和切换前做完整预检查，尽量避免部分项目已变更、部分项目失败的半完成状态；执行中失败时做回滚。
- `gpush` 和 `gmerge` 成功后输出清晰的分支解析和操作结果，避免用户操作错分支后无法确认。

## 目标

- 保留 `gmerge` 当前类似 `test/staging/prod` 的交互体验，但展示项由配置控制。
- 继续使用 `[projects.branches]` 作为项目级分支映射配置。
- 新增项目级分支输入别名，并且不和 `[projects.branches]` 冲突。
- 让 `gmerge`、`gswitch`、`gcreate`、`gpush` 共享一致的分支目标解析规则。
- 通过完整预检查减少频繁错误操作。
- 在成功摘要中展示输入目标、项目真实分支、远程分支等关键信息。
- 尽量兼容已有配置和现有行为。

## 非目标

- 不新增专门的分支预设或别名管理命令；用户通过配置文件维护。
- 不移除现有 `test`、`staging`、`prod` 分支配置支持。
- 不自动撤销远程 push；`gpush` 只汇总失败，不做远程回滚。
- 不改变现有工作区创建语义，除非需要抽取共享的分支名前缀处理逻辑。

## 配置设计

### 全局分支预设

在 `config.toml` 中新增配置：

```toml
[branch_presets]
test = "测试环境"
staging = "预发环境"
prod = "正式环境"
master = "主分支"
```

`branch_presets` 用于定义 `gmerge` 不带参数时展示的全局菜单和描述。

- key 是全局逻辑分支选项，例如 `test`。
- value 是终端展示描述，例如 `测试环境`。
- 展示顺序优先遵循 TOML 中的配置顺序；如果当前解析库不能保序，则使用稳定顺序并在文档中说明。
- 如果用户没有配置 `branch_presets`，Grove 默认保留当前内置选项：`test`、`staging`、`prod`。

命名使用 `branch_presets`，不建议使用 `git.origin`。原因是这里描述的是 Grove 层面的分支预设，不是 Git remote `origin`，避免概念混淆。

### 项目分支映射

继续使用 `[projects.branches]`，但从固定字段扩展为“必填 `main` + 可扩展 key-value 映射”：

```toml
[projects.branches]
main = "master"
test = "test-master"
staging = "pre"
prod = "master"
master = "main"
```

语义：

- `main` 仍然必填，用于普通工作区创建和 `gcreate` 的起始分支来源。
- 除 `main` 外的任意 key 都是项目内的逻辑分支名。
- value 是该项目中的真实 Git 分支名。
- 当用户希望全局预设能解析到项目特定分支时，`[projects.branches]` 的 key 应和 `[branch_presets]` 的 key 对齐。
- 如果某个全局预设 key 没有在项目 `[projects.branches]` 中配置，Grove 可以把该 preset key 本身当作真实分支名。
- 如果目标无法通过项目别名、项目分支映射、全局预设或真实分支 fallback 解析并验证成功，Grove 必须报告失败项目，并在任何有副作用的操作前整体中止。

已有只包含 `main`、`test`、`staging`、`prod` 的配置必须继续可读，并保持现有行为。

### 项目输入别名

每个项目新增可选配置：

```toml
[projects.branch_aliases]
test-master = "test"
```

语义：

- key 是命令输入别名。
- value 是项目内逻辑分支 key。
- 例如用户执行 `grove gmerge test-master`，Grove 先在 `[projects.branch_aliases]` 中把 `test-master` 解析为 `test`，再查 `[projects.branches].test`，最终得到真实 Git 分支 `test-master`。
- 每个项目可以独立配置自己的输入别名。
- 如果某个项目没有配置对应 alias，Grove 继续进入下一层解析。

## 分支目标解析规则

所有接受分支目标的命令应共享同一个解析器。

给定用户输入 `target` 和某个项目：

1. 先查 `[projects.branch_aliases].<target>`。如果命中，得到项目逻辑分支 key。
2. 再查 `[projects.branches].<逻辑分支 key 或 target>`。如果命中，得到真实 Git 分支名。
3. 如果 `<target>` 存在于全局 `[branch_presets]` 中，则使用 `<target>` 本身作为真实 Git 分支名。
4. 否则，在允许显式真实分支名的命令中，把 `<target>` 当作真实 Git 分支名。
5. 根据命令要求验证最终真实分支。已有分支类命令必须在产生副作用前确认分支存在。
6. 如果没有任何配置映射，并且显式真实分支也不存在，则返回错误，错误信息必须包含项目名和目标。

已确认的行为是：`gmerge test-master` 和 `gpush test-master` 允许显式真实分支名。也就是说，如果某个项目没有 alias 或 branch mapping，Grove 会尝试把 `test-master` 当作真实分支名；命令执行前仍要预检查这个分支是否存在。

## 命令行为

### `grove gmerge [target]`

不传 `target` 时，`gmerge` 展示全局 `branch_presets`：

```text
test     测试环境
staging  预发环境
prod     正式环境
master   主分支
```

传入 `target` 时，跳过菜单，直接按每个项目解析目标分支。

执行流程：

1. 检测当前 Grove 工作区；如果无法检测，则让用户选择。
2. 使用 `workspaces.toml` 中记录的工作区分支作为源分支输入。
3. 对每个项目分别解析源分支和目标分支。
4. 预检查所有项目工作区干净，并且解析后的源分支和目标分支都存在。
5. 每个项目在合并前先执行 fetch。
6. 对每个项目执行：
   - 记录操作前原始分支。
   - checkout 到解析后的目标分支。
   - 对目标分支执行远程 fast-forward pull。
   - 把解析后的源分支 merge 到目标分支。
   - checkout 回操作前原始分支。
7. 成功时按项目输出源分支、目标分支和用户输入目标的对应关系。
8. 如果 merge 失败，尽量切回该项目原始分支，并报告失败原因。

目标解析失败必须在任何 checkout 或 merge 前整体中止。

成功摘要需要让用户能确认没有合并错分支，例如：

```text
gmerge target: test

api: merged feature/login -> test-master (target: test)
web: merged feature/login -> develop (target: test)

Result: 2 succeeded, 0 failed
```

### `grove gswitch <target>`

`gswitch` 用于把当前工作区中的所有项目切换到按项目解析后的目标分支。

执行流程：

1. 检测当前 Grove 工作区；如果无法检测，则让用户选择。
2. 对每个工作区项目解析 `<target>`。
3. 预检查所有项目工作区干净，并且所有解析后的目标分支都存在。
4. 记录每个项目操作前原始分支。
5. checkout 到各自解析后的目标分支。
6. 所有项目都切换成功后，更新 `workspaces.toml` 中当前工作区的 `branch` 为用户输入的 `target`。
7. 如果任一 checkout 失败，尝试把已切换项目切回原始分支，并保持 `workspaces.toml` 不变。

工作区记录只在全部项目成功后更新。

### `grove gcreate <name>`

`gcreate` 用于在当前工作区所有项目中创建并切换到一个新分支。

分支命名：

- 使用现有 `git-prefix` 规则。
- `git-prefix` 中的日期模板必须展开。
- 如果配置了 `git-prefix`，并且输入没有以该前缀开头，则自动加上前缀。

起始分支：

- 新分支始终基于每个项目配置的 `main` 分支的最新远程版本创建。
- 每个项目必须先 fetch，再解析起点。
- Grove 优先使用 `origin/<main>`；如果远程起点不可用但本地 `<main>` 存在，可以 fallback 到本地 `<main>`。

执行流程：

1. 检测当前 Grove 工作区；如果无法检测，则让用户选择。
2. 使用 `git-prefix` 计算最终新分支名。
3. 预检查所有项目工作区干净。
4. 对所有项目执行 fetch。
5. 预检查新分支在所有项目中都不存在。
6. 预检查每个项目的 `main` 起点都能解析。
7. 记录每个项目操作前原始分支。
8. 基于解析后的 main 起点创建并切换到新分支。
9. 所有项目成功后，更新 `workspaces.toml` 中当前工作区的 `branch` 为新分支名。
10. 如果创建过程中任一项目失败，切回已变更项目的原始分支，并删除已创建的新分支。

预检查失败时不创建任何分支。执行中失败时做 best-effort 回滚。

### `grove gpush [target]`

不传 `target` 时，`gpush` 使用 `workspaces.toml` 中记录的工作区分支作为目标输入，并按每个项目解析。这样既保留普通同名分支工作流，也支持 `gswitch` 把工作区记录设置为逻辑预设或项目 alias 后继续默认推送。

传入 `target` 时，`gpush` 按每个项目解析目标：

- `branch_aliases` 可以把命令输入映射到项目逻辑分支。
- `[projects.branches]` 可以把逻辑分支映射到真实 Git 分支。
- `branch_presets` 可以允许某个预设 key 作为真实分支名使用。
- 如果目标没有命中配置，也可以作为真实分支名处理。

执行流程：

1. 检测当前 Grove 工作区；如果无法检测，则让用户选择。
2. 使用用户传入的 `target`；如果没有传入，则使用工作区记录的 `branch`。
3. 对每个项目解析 push 目标。
4. 预检查每个解析后的本地分支都存在。
5. 推送每个真实分支到同名远程分支，例如 `origin/<same-branch-name>`。
6. 成功时按项目输出用户输入目标、本地真实分支和远程目标分支。
7. push 失败按项目继续执行并最终汇总。

`gpush` 不做远程回滚。

成功摘要需要让用户能确认实际推送的分支，例如：

```text
gpush target: test

api: pushed test-master -> origin/test-master (target: test)
web: pushed develop -> origin/develop (target: test)

Result: 2 succeeded, 0 failed
```

## 预检查与回滚策略

会改变本地 checkout 状态的命令必须尽量避免部分变更：

- `gswitch` 和 `gcreate` 必须先预检查所有项目，再执行任何 checkout 或分支创建。
- `gmerge` 必须先预检查目标解析、分支存在和工作区干净，再执行 checkout 和 merge。
- `gpush` 必须先预检查目标解析和本地分支存在；push 本身失败时继续按项目汇总。

通用预检查：

- 能检测或选择当前工作区。
- 工作区中的每个项目仍存在于 `projects.toml`。
- 每个项目的 worktree 路径存在。
- 会切换分支的命令要求每个项目工作区干净。
- 每个目标都能解析。
- `gswitch`、`gmerge`、`gpush` 的目标分支必须已存在。
- `gcreate` 的新目标分支必须在所有项目中都不存在。
- `gcreate` 必须在 fetch 后能解析每个项目最新的 main 起点。

回滚规则：

- `gswitch`：把已切换项目切回记录的原始分支；保持工作区记录不变。
- `gcreate`：把已变更项目切回原始分支；尽可能删除已创建的新分支；保持工作区记录不变。
- `gmerge`：失败后尽量把失败项目切回原始分支。merge conflict 可能需要用户手动处理，必须明确提示。
- `gpush`：不回滚。

如果回滚本身失败，Grove 必须清晰列出需要用户手动恢复的项目。

## 用户可见错误

错误信息需要包含项目名和目标，便于定位：

- `api: cannot resolve branch target 'test-master'`
- `web: branch 'test-master' does not exist`
- `admin: new branch 'feature/foo' already exists`
- `api: working tree has uncommitted changes; commit or stash before switching`
- `service: cannot resolve main start point 'master' after fetch`

批量预检查失败时，Grove 应尽量展示所有发现的问题后再中止，让用户一次性修复。

## 兼容性

实现需要保留现有行为：

- 已有 `[projects.branches]` 中的 `main`、`test`、`staging`、`prod` 配置继续有效。
- 如果没有配置 `branch_presets`，`gmerge` 仍默认提供 `test`、`staging`、`prod`。
- 现有不带参数的 `gpush` 在普通同名分支工作流下继续推送工作区分支；当工作区记录是逻辑目标时，会额外通过新解析器解析。
- 现有 `gmerge` 行为可通过默认 presets 和项目分支映射继续保留。

由于 `[projects.branches]` 会变为可扩展结构，配置模型需要保留对必填 `main` 的直接访问，同时允许额外分支 key。

## 测试计划

单元测试：

- 分支目标解析器：
  - 项目输入 alias 能解析到逻辑分支，再解析到真实分支。
  - 能解析 `[projects.branches]` 中的直接 key。
  - 当项目没有 mapping 时，已配置的 `branch_presets` key 可作为真实分支名使用。
  - `gmerge`、`gpush`、`gswitch` 支持显式真实分支 fallback，并在分支存在时通过。
  - 无法解析时返回带项目名的错误。
- 配置序列化：
  - 能读取旧的 `main/test/staging/prod` 配置。
  - 能读取扩展分支 key。
  - 能读取可选 `branch_aliases`。
  - 能读取可选 `branch_presets`，缺失时使用默认值。

Git helper 测试：

- 如果命令代码需要运行时获取当前分支，则把当前仅用于测试的 current branch helper 改为运行时可用并测试。
- 检查分支存在。
- 基于起点创建分支。
- checkout 分支。
- 创建失败后删除分支。

命令级测试：

- `gmerge` 不带参数时展示 `branch_presets`。
- `gmerge <target>` 按项目解析；如果项目无法解析或目标分支不存在，在产生副作用前中止。
- `gmerge` 成功摘要展示输入目标、解析后的源分支和目标分支。
- `gswitch <target>` 只有在所有项目切换成功后才更新 `workspaces.toml`。
- `gcreate <name>` 先 fetch，基于最新配置 main 起点创建，并在成功后更新工作区记录。
- `gcreate` 预检查失败时不创建任何分支。
- `gcreate` 执行中失败时回滚已创建分支和原始 checkout。
- `gpush <target>` 解析到每个项目的真实分支并推送这些分支。
- `gpush` 成功摘要展示输入目标、本地真实分支和远程目标分支。

## 实现备注

- 当前 `BranchConfig` 是固定字段结构，需要谨慎改造为保留必填 `main`、同时允许额外 mapping 的结构。
- 当前 `git::current_branch` 只在测试下编译，回滚逻辑需要运行时可用的等价 helper。
- 当前 `gpush` 的 clap 定义不接受参数，需要新增可选 target 参数。
- 新命令 `gswitch` 和 `gcreate` 应遵循现有 `g*` 命令风格，并根据项目习惯添加短别名。
- 实现后必须更新 README，包括配置示例、命令参考、`gmerge/gpush/gswitch/gcreate` 行为说明，以及 `gpush/gmerge` 成功输出示例。
