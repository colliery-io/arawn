// The Arawn docs (ARAWN-T-0526): Astro Starlight with the Colliery theme
// starlight-aurora, in the four Diátaxis sections. Moved from mdBook.
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import mermaid from 'astro-mermaid';
import starlightAurora from 'starlight-aurora';
import mdbookRedirects, { MDBOOK_EXTRA } from 'starlight-aurora/mdbook-redirects';

// The Pages workflow sets DOCS_SITE and DOCS_BASE. On a pull request and
// locally they are empty, and the site is at /.
const site = process.env.DOCS_SITE || undefined;
const base = process.env.DOCS_BASE || '/';

const PROJECT = 'Arawn';
const REPO = 'https://github.com/colliery-io/arawn';

// The sidebar is written out, not generated: it keeps the order and the short
// labels of the old SUMMARY.md. Add a new page here.
const page = (slug, label) => ({ slug, label });

export default defineConfig({
	site,
	base,
	// Mermaid is one large chunk, loaded only on a page with a diagram.
	vite: { build: { chunkSizeWarningLimit: 4096 } },
	integrations: [
		// Before Starlight. Light diagrams in the light theme; dark diagrams in
		// the dark theme and in hyper.
		mermaid({ theme: 'dark', autoTheme: true }),
		// The old mdBook addresses (page.html) send the reader to the new page.
		// The old home page was intro.html.
		mdbookRedirects({ extra: { ...MDBOOK_EXTRA, 'intro.html': './' } }),
		starlight({
			title: PROJECT,
			description: 'A self-hosted personal agentic assistant in a single Rust binary.',
			social: [{ icon: 'github', label: 'GitHub', href: REPO }],
			editLink: { baseUrl: `${REPO}/edit/main/docs/` },
			plugins: [starlightAurora({ hyper: true })],
			sidebar: [
				{ label: 'Introduction', link: '/' },
				{
					label: 'Tutorials',
					items: [
						page('tutorials', 'Overview'),
						page('tutorials/first-chat', 'Your first chat session'),
						page('tutorials/first-lens', 'Your first lens'),
					],
				},
				{
					label: 'How-to guides',
					items: [
						page('how-to', 'Overview'),
						page('how-to/connect-google', 'Connect Google (Gmail, Calendar, Drive)'),
						page('how-to/connect-slack', 'Connect Slack'),
						page('how-to/connect-atlassian', 'Connect Atlassian (Jira + Confluence)'),
						page('how-to/connect-github', 'Connect GitHub'),
						page('how-to/connect-linear', 'Connect Linear'),
						page('how-to/create-a-feed', 'Create a feed'),
						page('how-to/bind-a-lens-to-a-feed', 'Bind a lens to a feed'),
						page('how-to/curate-a-lens', 'Curate a lens'),
						page('how-to/read-feeds-with-the-agent', 'Read feeds with the agent'),
						page('how-to/lock-down-permissions', 'Lock down permissions'),
						page('how-to/author-a-workflow-by-hand', 'Author a workflow by hand'),
						page('how-to/debug-oauth-failures', 'Debug OAuth failures'),
						page('how-to/recover-from-llm-warmup-failure', 'Recover from LLM warmup failure'),
						page('how-to/configure-hooks', 'Configure hooks'),
					],
				},
				{
					label: 'Reference',
					items: [
						page('reference', 'Overview'),
						page('reference/cli', 'CLI flags and subcommands'),
						page('reference/slash-commands', 'Slash commands'),
						page('reference/config-schema', 'Configuration schema'),
						page('reference/env-vars', 'Environment variables'),
						page('reference/llm-providers', 'LLM providers'),
						page('reference/data-directory', 'Data directory layout'),
						page('reference/troubleshooting', 'Troubleshooting'),
						page('reference/agent-tools', 'Agent tools'),
						page('reference/permissions', 'Permission model'),
						page('reference/shell-sandbox', 'Shell sandbox'),
						page('reference/integrations', 'Integrations'),
						page('reference/integrations-config', 'Integrations config'),
						page('reference/lens-cli', 'Lens CLI'),
						page('reference/lens-tools', 'Lens tools'),
						page('reference/steward-subroutines', 'Steward subroutines'),
						page('reference/ceremonies-tools', 'Ceremonies tools'),
						page('reference/todos-tools', 'Todos tools'),
						page('reference/feeds-overview', 'Feeds overview'),
						page('reference/feed-templates', 'Feed templates'),
						page('reference/feed-search-tool', 'feed_search tool'),
						page('reference/projection-tables', 'Projection tables'),
						page('reference/palace-types', 'Palace types'),
						page('reference/memory-model', 'Memory model'),
						page('reference/workflow-tools', 'Workflow tools'),
						page('reference/plugins', 'Plugins'),
						page('reference/mcp', 'MCP'),
						page('reference/skills', 'Skills'),
						page('reference/sub-agents', 'Sub-agents'),
					],
				},
				{
					label: 'Explanation',
					items: [
						page('explanation', 'Overview'),
						page('explanation/what-is-arawn', 'What is arawn?'),
						page('explanation/the-agent-loop', 'The agent loop'),
						page('explanation/three-layer-data-model', 'The three-layer data model'),
						page('explanation/feeds', 'Feeds'),
						page('explanation/projections', 'Projections'),
						page('explanation/palaces', 'Palaces'),
						page('explanation/extraction', 'Extraction'),
						page('explanation/steward', 'The Steward'),
						page('explanation/lenses', 'Lenses'),
						page('explanation/identity-by-lens', 'Identity by lens'),
						page('explanation/memory-design', 'Memory design'),
						page('explanation/workflows', 'Workflows'),
						page('explanation/permission-model', 'Permission model'),
						page('explanation/ceremonies', 'Ceremonies'),
						page('explanation/integrations-overview', 'Integrations overview'),
						page('explanation/oauth-primer', 'OAuth primer'),
					],
				},
				{
					label: 'Contributing',
					items: [
						page('contributing/integration-docs', 'Maintaining integration docs'),
					],
				},
			],
		}),
	],
});
