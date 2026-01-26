# Discovery Tree Workflow

**Just-in-time planning through hierarchical task breakdowns**

## Core Principle

Start with minimal details and expand as you work. Don't plan everything upfront - let the work inform the plan.

## Just-in-Time Planning

- Begin with a simple goal
- Break down only what you need to start
- Discover and add tasks as complexity emerges
- Delay detailed planning until actually needed

## Emergent Work

As you work, three things happen:

1. **Unexpected Complexity**: If a task is harder than expected, break it into subtasks
2. **New Requirements**: Add newly discovered work to the tree
3. **Distractions**: Capture unrelated ideas as separate lower-priority tasks

## Structure

Every discovery tree requires:

- **Epic**: A container that tracks overall completion percentage
- **Root Task**: Describes the actual user value being delivered

Tasks are organized hierarchically with parent-child relationships.

## Visual Status

Tasks are color-coded by status for at-a-glance understanding:

- **open**: Not started, ready to work on
- **in_progress**: Currently being worked on
- **closed**: Completed
- **blocked**: Waiting on dependencies

## Workflow Cycle

1. **Create epic and root task**
2. **Initial breakdown**: 2-10 minute planning conversation to identify first tasks
3. **Claim a ready task**: Find unblocked work and start
4. **Work and discover**: As complexity emerges, create subtasks
5. **Update and close**: Mark progress, close completed tasks
6. **Check progress**: Review epic status and repeat

## API Functions

The workflow uses these conceptual operations:

- `setWorkspace()` - Initialize working context
- `createTask(description, priority)` - Add new tasks
- `addDependency(parent, child)` - Establish hierarchy
- `updateTask(id, status, notes)` - Modify task state
- `closeTask(id)` - Mark completion
- `findReadyTasks()` - Identify unblocked work
- `drawTree()` - Visualize the hierarchy
- `getEpicStatus()` - Track completion metrics

## Priority Levels

Tasks have priority (default: medium):
- **high**: Critical path work
- **medium**: Normal workflow
- **low**: Nice-to-have or captured distractions

## Integration with Other Practices

**With TDD**: Each test cycle informs task breakdown. A failing test might reveal subtasks needed.

**With Example-Driven Design**: Examples map to phases of tree growth.

**With Mikado Method**: Prerequisites become discoverable dependencies in the tree.

## Key Benefits

- **No premature planning**: Avoid wasting time planning things that change
- **Captures discovery**: New work gets added as you learn
- **Visual progress**: Status colors show momentum at a glance
- **Handles complexity**: Break down only when needed
- **Prevents overwhelm**: Focus on ready tasks, not the whole tree

## Anti-Patterns

❌ Planning all tasks in detail upfront
❌ Ignoring discovered complexity (not breaking down)
❌ Working on blocked tasks
❌ Forgetting to update status as you work
❌ Not capturing distractions (letting them derail you)

## Philosophy

Work is a discovery process. You learn what needs doing by doing. The tree grows as understanding grows.

Start simple. Break down just-in-time. Let the work guide the plan.
