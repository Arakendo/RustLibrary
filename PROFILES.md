# Adoption Profiles

Profiles are recommended subsets of the standard fixture, not separate copies.
Keeping one canonical set of templates prevents profile variants from drifting.
Any profile may also adopt selected files from [`root-files/`](root-files/).
Any profile may adopt the optional [`corpus/`](corpus/) package when the project
uses executable architectural evidence.

## Minimal

For a small project establishing its first durable documentation:

```text
docs/
  README.md
  product/
  design-document.md
  roadmap.md
  testing-strategy.md
  adr/
```

Add plans and notes when work outgrows issues or the project needs preserved
investigation records.

## Library

For a reusable package, SDK, framework, or command-line tool:

```text
minimal profile
+ architecture-reviews/
+ design/
+ guides/
+ tutorials/
+ reference/
+ governance/
+ security/
+ quality/
+ changes/
+ audits/
+ plans/
+ notes/
+ releases/
+ support/
+ archive/
```

Keep compatibility, migration, deprecation, security, quality, release, and
support records proportional to the library's actual commitments. Operations
and incident records are usually unnecessary unless the project also runs
hosted services.

## Service

For a deployed application or stateful system, use the full fixture:

```text
standard fixture
+ operations/runbooks/
+ operations/incidents/
```

Add project-specific monitoring, service objectives, backup and restore,
disaster recovery, and release procedures under `operations/`.

## Research Or Prototype

Start with:

```text
docs/
  README.md
  product/
  design-document.md
  notes/
  conversations/
  plans/
  archive/
```

Introduce ADRs and reviews when decisions become durable constraints. Avoid
presenting exploratory observations as settled architecture.

## Internal Tool

Start with the minimal profile, then usually add:

```text
+ guides/
+ reference/
+ governance/
+ support/
```

Add security, operations, releases, and change management according to the
tool's data sensitivity, deployment model, and support expectations.

## Architectural Corpus Extension

Add this extension to any profile when executable examples, compatibility
matrices, provider observations, or multi-component scenarios are used to
pressure architectural claims:

```text
corpus/
  README.md
  campaigns/
    README.md
    TEMPLATE.md
```

Keep executable test code in its owning test project unless the project has a
clear reason to make a corpus entry independently runnable. Campaign folders
own context, inputs, structured observations, provenance, and run instructions.
