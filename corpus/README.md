# {{PROJECT_NAME}} Architectural Corpus

The `corpus/` directory contains executable evidence used to investigate and
regress architectural claims. It is deliberately separate from examples,
ordinary tests, and product fixtures: a corpus entry exists to pressure a
specific ownership boundary, compatibility surface, or Architectural Review
question.

Corpus observations are evidence, not policy. They do not override current
design documents or accepted ADRs. When a question is unresolved, assertions
must describe observed behavior without silently promoting it into a supported
contract.

## Primary Question

A useful corpus campaign asks:

> Can {{PROJECT_NAME}} express this behavior naturally through the intended
> ownership and dependency boundaries?

## Initial Shape

```text
corpus/
  README.md
  campaigns/
    README.md
    TEMPLATE.md
    <campaign>/
      README.md
      fixtures/            optional, campaign-owned inputs
      evidence.*           optional structured observations
```

Start with campaigns. Add focused proofs, consumer applications, shared
incubating libraries, or asset areas only after repeated evidence demonstrates
a stable need for those categories.

## Admission Checklist

Before adding a campaign, record:

- one primary architectural question or accepted claim under test;
- the ownership boundary or compatibility surface under pressure;
- why an ordinary unit or integration test is insufficient by itself;
- observable success and failure conditions;
- whether each assertion is contract evidence or only an observation;
- dependencies, platforms, environmental requirements, and non-goals;
- the AR, ADR, plan, roadmap item, or design section that consumes the evidence;
- deterministic inputs and cleanup requirements where practical.

An entry that cannot answer these questions is still an experiment. Give it a
focused claim before making it a permanent corpus campaign.

## Corpus Versus Tests And Goldens

| Artifact | Primary question |
| --- | --- |
| Unit or contract test | Does this bounded behavior satisfy its contract? |
| Integration test | Do named components work together correctly? |
| Golden or snapshot | Does output match a reviewed expectation? |
| Corpus campaign | What does sustained executable evidence say about an architectural claim? |

A test fixture does not become architectural evidence merely because it is
referenced by a campaign. A corpus observation does not become a reviewed
golden or supported contract implicitly.

Executable corpus adapters should normally run through the relevant test
project. Campaign folders own context, inputs, structured observations, and run
instructions; test projects own executable test code.

## What Does Not Belong Here

- Arbitrary scratch programs
- Generated build output
- Unrelated product applications
- Benchmarks with no architectural claim
- Ordinary test fixtures with no campaign context
- Generated output without explicit provenance and review policy
- Duplicate campaigns that pressure no new boundary

## Findings And Decisions

Corpus work may show that the current architecture works, needs refinement, or
is incomplete. Preserve all three outcomes as evidence.

```text
Corpus campaign
    -> observed evidence
    -> Architectural Review when ownership or contract is uncertain
    -> ADR when an architectural change is accepted
    -> Plan for implementation and validation
```

Repeated friction is evidence for review, not automatic permission to promote
an abstraction or supported contract.

## Regression Role

Once accepted, a corpus campaign becomes a regression artifact. Refactors
should preserve its primary claim unless an AR or ADR deliberately changes that
claim. Put automated assertions at the narrowest honest boundary and retain
machine-specific observations with explicit provenance.
