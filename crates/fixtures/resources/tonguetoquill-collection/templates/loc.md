---
QUILL: usaf_memo
letterhead_title: DEPARTMENT OF THE AIR FORCE
letterhead_caption:
  - 20th Fighter Wing (ACC)
  - Shaw Air Force Base South Carolina
memo_for:
  - <<subject_rank>> <<subject_first_name>> <<subject_middle_initial>>. <<subject_last_name>>
memo_from:
  - <<issuer_org_symbol>>
subject: Letter of Counseling
signature_block:
  - <<issuer_first_name>> <<issuer_middle_initial>>. <<issuer_last_name>>, <<issuer_rank>>, USAF
  - <<issuer_duty_title>>
tag_line: Get Rekt # Optional tagline

# Tune to fit all indorsements on second page
font_size: 11
compress_indorsements: true

# Based on LOCAR template from https://www.shaw.af.mil/Portals/98/Docs/Legal/Revamp/2022%20LOCAR%20TEMPLATE%20(Enlisted).docx
---

**NOTE: This template is undergoing major refinement. It is included for demonstration purposes.**

<!-- EDIT: Describe the incident: what the member did or failed to do, citing specific incidents and dates, and if possible, with UCMJ articles. -->
Investigation has disclosed the <<incident_description>>.

<!-- EDIT: Counseling paragraph: Discuss the impact of the member's actions and what improvement is expected. -->
You are hereby counseled.
<<impact_and_expected_improvement>>.
Your conduct is unacceptable and any future misconduct may result in more severe action.

The following information required by the Privacy Act is provided for your information. **AUTHORITY**: 10 U.S.C. § 8013. **PURPOSE**: To obtain any comments or documents you desire to submit (on a voluntary basis) for consideration concerning this action. **ROUTINE USES**: Provides you an opportunity to submit comments or documents for consideration. If provided, the comments and documents you submit become a part of the action. **DISCLOSURE**: Your written acknowledgement of receipt and signature are mandatory. Any other comments or documents you provide are voluntary.

You will acknowledge receipt of this letter immediately by signing the first indorsement. Within 3 duty days from the day you received this letter, you will provide your response by signing the second indorsement below. Any comments or documents you wish to be considered concerning this letter must be submitted at that time, and will become part of the record, consistent with AFI 36-2907, Adverse Administrative Actions, paragraph 2.4.2.5. After receiving your response, I intend to notify you of my final disposition of this action within 3 duty days.

---
CARD: indorsement
for: <<issuer_org_symbol>>
from: <<subject_rank>> <<subject_first_name>> <<subject_middle_initial>>. <<subject_last_name>>
signature_block: <<subject_first_name>> <<subject_middle_initial>>. <<subject_last_name>>, <<subject_rank>>, USAF
format: separate_page # Omit to keep on same page
---

<!-- EDIT: First Indorsement: Subject acknowledges receipt of the LOC -->
I acknowledge receipt and understanding of this letter on ________________ at ___________ hours. I understand that I have 3 duty days from the date I received this letter to provide a response and that I must include in my response any comments or documents I wish to be considered concerning this Letter of Counseling.

---
CARD: indorsement
from: <<subject_rank>> <<subject_first_name>> <<subject_middle_initial>>. <<subject_last_name>>
for: <<issuer_org_symbol>>
signature_block: <<subject_first_name>> <<subject_middle_initial>>. <<subject_last_name>>, <<subject_rank>>, USAF
date: "Date: _____________"
---

<!-- EDIT: Second Indorsement: Subject's response - choose one option -->
I have reviewed the allegations contained in this Letter of Counseling. _I am submitting the attached documents in response_ / _I hereby waive my right to respond_.

---
CARD: indorsement
from: <<issuer_rank>> <<issuer_first_name>> <<issuer_middle_initial>>. <<issuer_last_name>>
for: <<subject_rank>> <<subject_first_name>> <<subject_middle_initial>>. <<subject_last_name>>
signature_block: <<issuer_first_name>> <<issuer_middle_initial>>. <<issuer_last_name>>, <<issuer_rank>>, USAF
date: "Date: _____________"
---

<!-- EDIT: Third Indorsement: Issuer's final decision - choose appropriate options -->
_I have considered the response you submitted_ / _You waived your right to submit a response to this action_. I have decided to _withdraw_ / _sustain_ the Letter of Counseling.

---
CARD: indorsement
from: <<subject_rank>> <<subject_first_name>> <<subject_middle_initial>>. <<subject_last_name>>
for: <<issuer_org_symbol>>
signature_block: <<subject_first_name>> <<subject_middle_initial>>. <<subject_last_name>>, <<subject_rank>>, USAF
date: "Date: _____________"
---

<!-- EDIT: Fourth Indorsement: Subject acknowledges final decision -->
I acknowledge receipt of the final decision regarding disposition of this Letter of Counseling on ____________ at __________ hours.