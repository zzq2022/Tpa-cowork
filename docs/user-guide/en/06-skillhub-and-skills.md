# 06 · SkillHub and Skill Market

**SkillHub** and **MySkills** provide TPA CoWork users with comprehensive AI skill extension capabilities. You can explore, preview, and download the latest Agent skills shared by the community and team from the cloud SkillHub market, as well as manage locally installed skills, create custom skills, and submit them to the cloud for publishing and review.

**Entrances**:
- "SkillHub" icon on the left sidebar (Cloud Skill Market)
- "My Skills" icon on the left sidebar (Local skill management and cloud sync)
- Settings → "Skills" panel

**Chapter Overview**

- [6.1 SkillHub Cloud Skill Market](#61-skillhub-cloud-skill-market)
- [6.2 Search, Filter, and Skill Details](#62-search-filter-and-skill-details)
- [6.3 One-Click Download and Installation](#63-one-click-download-and-installation)
- [6.4 MySkills Management](#64-myskills-management)
- [6.5 Skill Creation and Visual Editor](#65-skill-creation-and-visual-editor)
- [6.6 Publishing Skills to SkillHub for Review](#66-publishing-skills-to-skillhub-for-review)
- [6.7 Skill Version Upgrades and Freshness Detection](#67-skill-version-upgrades-and-freshness-detection)
- [6.8 Skill Evolution and Auto Curator](#68-skill-evolution-and-auto-curator)

---

## 6.1 SkillHub Cloud Skill Market

SkillHub is the cloud-based Agent skill discovery and sharing platform for TPA CoWork. Through SkillHub, the AI assistant is no longer restricted to a fixed initial feature set, but can obtain on-demand capabilities (such as specialized coding tools, document processing, business process automation, etc.).

### Cloud Account & Session Login
- You can sign in or register a SkillHub platform account in the top-right corner of the SkillHub view or via Settings → "Profile".
- Upon logging in, your token is securely stored locally with automatic refresh and safe sign-out capabilities.
- Cloud login unlocks skill submission, publishing status tracking, and private/team skill synchronization.

---

## 6.2 Search, Filter, and Skill Details

Click **SkillHub** on the left sidebar to open the cloud market view:

1. **Multi-dimensional Category Filtering**: Top category tabs (e.g., Tools, Developer Utilities, Office Automation, Data Processing) allow quick filtering.
2. **Keyword Search**: Search by skill name, author, or description in real time.
3. **Popularity & Sorting**: Sort by "Most Popular" (Downloads), "Highest Rated" (Stars), "Most Viewed", and "Recently Updated".
4. **Skill Card & README Preview**:
   - Click any skill card to open the **Skill Details modal**.
   - The details page renders formatted Markdown documentation (`SKILL.md` / `README.md`), displaying applicable scenarios, trigger commands, author username, update timestamps, and version tags.

---

## 6.3 One-Click Download and Installation

Once you find a skill you need:
1. Click the **"Download / Install"** button on the card or detail page.
2. The system automatically fetches and extracts the skill archive to `~/.tpa-cowork/skills/<skill-slug>/` on your local machine.
3. If an existing version or duplicate slug is found, the system prompts for upgrade confirmation.
4. After downloading, the skill takes effect immediately in your local environment without requiring an application restart.

---

## 6.4 MySkills Management

Click **My Skills** on the left sidebar to access local skill management:

- **Local Skill Overview**: Displays all currently installed skills (including built-in preset skills and custom/downloaded skills).
- **Enable / Disable Toggle**: Easily toggle skills on or off. Disabled skills will not load prompt instructions or tool capabilities into conversations.
- **Inspect & Configure**: View internal file structures, referenced tools, and configuration options.

---

## 6.5 Skill Creation and Visual Editor

Want to teach your AI assistant custom workflows? Create a new skill directly in "My Skills":

1. Click the **"New Skill"** button at the top.
2. Fill in metadata: Skill Slug, Display Name, Category, Version, and Description.
3. Use the structured Markdown editor to compose `SKILL.md`:
   - Write system prompt instructions (defining when and how the AI should respond).
   - Reference system tools or custom scripts.
4. Save to apply the skill locally immediately.

---

## 6.6 Publishing Skills to SkillHub for Review

To share a skill with your team or the community:

1. Select your custom skill in "My Skills".
2. Click **"Publish to SkillHub"**.
3. The system packs the metadata and `SKILL.md` to submit to the SkillHub cloud server.
4. The skill status shifts to **Pending Review (`pending-review`)**.
5. Once approved, the status updates to **Published (`published`)** and becomes searchable globally. Rejection reasons (`rejected`) will be provided if changes are required.

---

## 6.7 Skill Version Upgrades and Freshness Detection

- The background periodically checks for cloud version updates against installed local skills.
- When an update is available, an **"Update Available"** badge appears in the skill list.
- Click **"One-Click Update"** to instantly update to the latest cloud version.

---

## 6.8 Skill Evolution and Auto Curator

Beyond manual creation and cloud downloads, TPA CoWork features a unique **"Skill Evolution"** mechanism. As you chat and work with the AI daily, it automatically perceives recurring patterns and distills, consolidates, and purifies these experiences into new skills.

**Entrance**: Settings → Skills → **"Self-Evolution"** tab.

### 1. Automatic Dialogue Distillation
- **Automated Analysis**: Whenever frequent tool calls, long workflows, or user corrections occur in a session, background analysis modules inspect the conversation.
- **Draft Skill Generation**: The system extracts valuable action steps, command parameters, and prompt patterns to construct a `SKILL.md` file, which drops into the **"Pending Draft Skills"** list.

### 2. Draft Clustering and Auto-Curator
- **Topic De-duplication**: As conversations accumulate, multiple draft skills with overlapping topics might be created. The **Auto-curator** regularly scans drafts to identify clusters of redundant or similar skills.
- **One-Click Merge & Purification**: It proposes merging proposals (e.g., "Keep 1 core skill and delete remaining duplicates"), preventing skill library bloat and conflicts.

### 3. Quality Gates & Auto-Activation
- **Quality Gate Inspection**: During distillation, hollow content lacking concrete commands, simple session recaps, or low reuse probability skills are automatically discarded.
- **Auto-Activation Control**:
  - **Off (Default)**: Distilled skills land in the pending drafts area for manual user review and activation.
  - **On**: Verified skills automatically activate into system prompts, delivering a self-learning experience that grows more tailored to you over time.

---

## Next Steps

- Manage AI tools and permissions → [07 Tools and Permissions](07-tools-and-permissions.md)
- Configure external tool integrations → [11 Connect and Extend](11-connect-and-extend.md)
