---
# Metadata
examinee:
  last: Fry
  first: Phillip
  middle: J.
  grade: SrA
# Section 1
dod_id: 1999123101
date_completed: 3000-01-01
eligibility_period: 4000-01-01
organization: Planet Express
location: New New York
mds: F-9000
crew_position: Delivery Boy
# Section 2
requisite_info:
  - requisite: expendable
    date: 3000-01-01
    results: Q1
  - requisite: good attitude
    date: 3000-01-01
    results: Q1
# Section 3
eval_info:
  - evaluation: I'm laying you all off, effective immediately. Except you, Fry. You're fired.
    date: 3000-01-01
  - evaluation: Oh, Fry. I love you, but you're a man. And a stupid man at that.
    date: 3000-01-01
  - evaluation: Of all the friends I've had... you're the first.
    date: 3000-01-01
# Section 4
qual_level:
  qualified: asdf
  unqualified: X
  expiration: N/A
# Section 5
addi_training:
  due_dates: 3000-01-01
  completed_dates: 3000-01-01
  certifying: Turanga Leela, Capt, Planet Express
# Section 6
other:
  restrictions: X
  exceptionally_qualified:
  downgrade: X
# Section 7
flight_examiner:
  name: Capt Turanga Leela
  org: Planet Express
  check.concur: X
reviewing_officer:
  name: Hermes Conrad
  org: Planet Express
  check.concur: X
final_officer:
  name: Professor Farnsworth
  org: Planet Express
  check.concur: X
# Section 9
comments:
  restrictions: You are grounded
  #exceptionally_qualified:
examiner_remarks:
  mission_description: Delivering packages to all points in the universe.
  discrepancies: |
    Your delivery times are consistently late.
    You smell like burning hair.
    I'm sure you did your best, which is to say you failed miserably
    Maybe you should stick to the one thing you're good at - nothing
  addi_training: You need to learn how to not be you
  addi_comments: None
reviewing_remarks: None
final_remarks: None
addi_reviews: “None”
---

whatever

---
# We can also append to arrays with scoped metadata
SCOPE: requisite_info
requisite: works for low pay
date: 3000-01-01
results: Q1
---

---
SCOPE: eval_info
evaluation: This man is a lazy, good-for-nothing, loafing...
date: 3000-01-01
---