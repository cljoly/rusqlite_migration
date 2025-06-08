// SPDX-License-Identifier: Apache-2.0
// Copyright Clément Joly and contributors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use insta::assert_snapshot;

use crate::tests::helpers::{all_valid_down, m_valid0_down, m_valid0_up, m_valid_fk_down};
use crate::*;

#[test]
fn test_m_display() {
    insta::assert_snapshot!("up_only", m_valid0_up());
    insta::assert_snapshot!("up_only_alt", format!("{:#}", m_valid0_up()));

    insta::assert_snapshot!("up_down", m_valid0_down());
    insta::assert_snapshot!("up_down_alt", format!("{:#}", m_valid0_down()));

    insta::assert_snapshot!("up_down_fk", m_valid_fk_down());
    insta::assert_snapshot!("up_down_fk_alt", format!("{:#}", m_valid_fk_down()));

    let everything = M {
        up: "UP",
        up_hook: Some(Box::new(|_: &Transaction| Ok(()))),
        down: Some("DOWN"),
        down_hook: Some(Box::new(|_: &Transaction| Ok(()))),
        foreign_key_check: true,
        comment: Some("Comment, likely a filename in practice!"),
    };
    insta::assert_snapshot!("everything", everything);
    insta::assert_debug_snapshot!("everything_debug", everything);
    insta::assert_compact_debug_snapshot!("everything_compact_debug", everything);
    insta::assert_snapshot!("everything_alt", format!("{everything:#}"));
}

#[test]
fn display_simple() {
    assert_snapshot!(Migrations::new(all_valid_down()));
}

#[test]
fn display_alternate() {
    assert_snapshot!(format!("{:#}", Migrations::new(all_valid_down())));
}
