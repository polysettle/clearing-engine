# Governance

## Project Structure

**clearing-engine** is maintained under the OpenSettlement organization.

### Roles

**Maintainer** — Full commit access, release authority, RFC approval. Currently a single-maintainer project with plans to expand as the community grows.

**Contributor** — Anyone who submits accepted pull requests. Listed in CONTRIBUTORS.md after first merged PR.

**Reviewer** — Trusted contributors invited to review PRs. Earn this role through consistent, high-quality contributions.

### Decision Making

- **Minor changes** (bug fixes, docs, small improvements): Maintainer review + merge.
- **Significant changes** (new modules, API changes, algorithm changes): RFC process required. 7-day discussion period. Maintainer makes final decision, documented in the RFC.
- **Breaking changes**: Require RFC + semantic version bump + migration guide.

### Versioning

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Incompatible API changes
- **MINOR**: New functionality, backwards compatible
- **PATCH**: Bug fixes, backwards compatible

Pre-1.0: API is not yet stable. Minor versions may include breaking changes with documentation.

### Release Process

1. All CI checks pass on `main`
2. Changelog updated
3. Version bumped in `Cargo.toml`
4. Tagged release created on GitHub
5. Published to crates.io

### Code of Conduct

Be professional. This is infrastructure. Discussions should be technical, constructive, and focused on the problem domain.

We do not tolerate harassment, discrimination, or personal attacks. Violations result in removal from the project.

### Neutrality Policy

This project is **sovereign-agnostic** and **politically neutral**.

- Code, documentation, and discussions must not advocate for or against specific countries, political blocs, or geopolitical positions.
- The engine is designed to work with any currency, any party, any settlement system.
- Marketing or positioning of the project in political contexts by contributors is discouraged.

This neutrality is a feature, not a limitation. It is what makes the project adoptable by any institution.

### Future Governance

As the project grows, governance will evolve:

- **5+ regular contributors**: Establish a core team with shared review responsibilities.
- **Institutional adoption**: Consider forming a technical steering committee.
- **Foundation**: If warranted, explore fiscal sponsorship or foundation structure.

Governance changes will be proposed via RFC.
