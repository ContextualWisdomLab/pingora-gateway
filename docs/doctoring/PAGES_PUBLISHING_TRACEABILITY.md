# GitHub Pages publication traceability

## Decision

The buyer-facing documentation site is a publication surface, not release evidence by itself. The repository may publish only an exact protected `main` revision through the `github-pages` deployment environment. A checked-in `docs/index.md`, a successful Jekyll build, or an uploaded Pages artifact is insufficient: the deploy job must fetch the public site and verify that `/source-sha.txt` equals the exact GitHub source revision used for the build.

The first publication lane is intentionally `workflow_dispatch` only. Repository metadata reported `has_pages: false` on 2026-09-12 and the available repository writer does not expose GitHub Pages administration. Automatic `push` deployment would therefore turn every protected-main integration into an avoidable failed deployment before the Pages source is enabled. Once the repository owner enables GitHub Actions as the Pages source and the first exact protected revision is published and verified, automatic protected-main publication can be considered separately.

## Constraints and alternatives

- Publishing a PR head was rejected because a mutable review branch is not protected product documentation authority.
- Publishing directly from a branch/folder without an exact build/deploy receipt was rejected because it does not prove which rendered public content is live.
- `actions/configure-pages` `enablement: true` was rejected for this workflow: the action documents that automatic enablement requires a token other than `GITHUB_TOKEN`, with repository administration and Pages write authority for a GitHub App. Secret/admin authority is not copied into the repository merely to avoid the owner-admin handoff.
- The workflow uses GitHub's Jekyll Pages builder because the buyer source is Markdown under `docs/`; uploading the raw directory would not prove the rendered site that users receive.
- Deployment concurrency does not cancel an in-flight publish. A later publish may queue, but the current production deployment is allowed to finish so the externally observed source identity remains attributable.
- GitHub's Pages deployment documentation requires `pages: write` and `id-token: write` on the deploy job. Those privileges are therefore scoped to `deploy`; the build job has only `contents: read` and `pages: read`, while workflow-level permissions are empty. This prevents the build/Jekyll steps from minting an OIDC token or creating a Pages deployment.

## Exact action authority

The workflow pins GitHub-owned actions by commit rather than mutable major tags. The major tags were revalidated immediately before this lane was written.

| Purpose | Release line | Exact commit |
| --- | --- | --- |
| Checkout | `actions/checkout@v6` | `d23441a48e516b6c34aea4fa41551a30e30af803` |
| Pages metadata/configuration | `actions/configure-pages@v6` | `45bfe0192ca1faeb007ade9deae92b16b8254a0d` |
| Jekyll Pages build | `actions/jekyll-build-pages@v1` | `44a6e6beabd48582f863aeeb6cb2151cc1716697` |
| Pages artifact upload | `actions/upload-pages-artifact@v5` | `fc324d3547104276b827a68afc52ff2a11cc49c9` |
| Pages deployment | `actions/deploy-pages@v5` | `368f82528645a54fb793d4d04e342629a3f51346` |

`configure-pages@v6` uses Node.js 24. `upload-pages-artifact` latest release observed for this decision was v5.0.0, published 2026-04-10. These observations are inputs to the exact pin, not permission to float those tags later.

## Acceptance evidence

Source-level acceptance is encoded by `tests/pages_workflow_contract.rs` and requires:

- manual dispatch with an explicit `refs/heads/main` fail-closed guard;
- exact `github.sha` checkout and post-checkout identity verification;
- empty workflow-level permissions, build-only `contents: read` + `pages: read`, and deploy-only `pages: write` + `id-token: write`;
- exact-SHA action pins;
- Jekyll build from `./docs` into `./_site`;
- a generated `source-sha.txt` containing the protected source SHA;
- Pages artifact upload, deployment through `github-pages`, and public HTTPS verification of both the source marker and site root; and
- serialized deployment without cancelling an in-progress publication.

Operational completion additionally requires repository-owner administration to enable GitHub Pages with GitHub Actions as the publishing source, followed by a successful manual run from protected `main`. Record the deployment run, protected source SHA, returned public URL, and the public `/source-sha.txt` value. Until that happens, the Pages gap remains open.

## Recovery

A bad documentation deployment is recovered through normal protected-branch history: repair or revert the documentation by reviewed forward change on `main`, then dispatch the Pages workflow again. Do not force-move the protected branch or publish an arbitrary historical PR head. The public identity check must converge to the repaired protected SHA before recovery is declared complete.

## References

GitHub. (n.d.). *Configuring a publishing source for your GitHub Pages site*. GitHub Docs. https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site

GitHub. (n.d.). *Using custom workflows with GitHub Pages*. GitHub Docs. https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages

GitHub. (n.d.). *Workflow syntax for GitHub Actions: Defining access for the GITHUB_TOKEN scopes*. GitHub Docs. https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax

GitHub. (2026). *actions/configure-pages v6 action metadata* [Source code, commit 45bfe0192ca1faeb007ade9deae92b16b8254a0d]. https://github.com/actions/configure-pages/blob/45bfe0192ca1faeb007ade9deae92b16b8254a0d/action.yml

GitHub. (2026). *actions/deploy-pages* [Documentation, v5 line]. https://github.com/actions/deploy-pages

GitHub. (2026, April 10). *actions/upload-pages-artifact v5.0.0*. https://github.com/actions/upload-pages-artifact/releases/tag/v5.0.0
