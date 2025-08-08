// moon: The build system and package manager for MoonBit.
// Copyright (C) 2024 International Digital Economy Academy
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// For inquiries, you can contact us via e-mail at jichuruanjian@idea.edu.cn.

//! Compiler command abstraction

mod build_common;
mod build_package;
mod check;
mod link_core;

use std::borrow::Cow;
use std::path::Path;

use crate::model::{BuildTarget, TargetKind};
use crate::pkg_name::PackageFQN;

pub use self::build_common::BuildCommonArgs;
pub use self::build_package::MooncBuildPackage;
pub use self::check::MooncCheck;
pub use self::link_core::MooncLinkCore;

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorFormat {
    Regular,
    Json,
}

#[derive(Clone, Debug)]
pub struct MiDependency<'a> {
    pub path: Cow<'a, Path>,
    pub alias: Option<Cow<'a, str>>,
}

impl<'a> MiDependency<'a> {
    pub fn to_alias_arg(&self) -> String {
        if let Some(alias) = &self.alias {
            format!("{}:{}", self.path.display(), alias)
        } else {
            format!("{}:{}", self.path.display(), self.path.display())
        }
    }

    pub fn new(path: impl Into<Cow<'a, Path>>, alias: impl Into<Cow<'a, str>>) -> Self {
        Self {
            path: path.into(),
            alias: Some(alias.into()),
        }
    }

    #[allow(unused)]
    pub fn no_alias(path: impl Into<Cow<'a, Path>>) -> Self {
        Self {
            path: path.into(),
            alias: None,
        }
    }
}

/// Represents a package name of a specific kind passed to the compiler.
/// Used to create the actual package name of the compiled package.
///
/// Since tests are not dependencies of any other packages, adding a suffix to
/// test packages will not interfere with the names of other packages.
#[derive(Clone, Debug)]
pub struct CompiledPackageName<'a> {
    pub fqn: &'a PackageFQN,
    pub kind: TargetKind,
}

impl<'a> CompiledPackageName<'a> {
    pub fn new(fqn: &'a PackageFQN, target: BuildTarget) -> Self {
        Self {
            fqn,
            kind: target.kind,
        }
    }
}

impl<'a> std::fmt::Display for CompiledPackageName<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let suffix = match self.kind {
            TargetKind::Source => "",
            TargetKind::WhiteboxTest => "__wb_test",
            TargetKind::BlackboxTest => "__bb_test",
            TargetKind::InlineTest => "__inline_test",
            TargetKind::SubPackage => "",
        };
        write!(f, "{}{}", self.fqn, suffix)
    }
}

#[derive(Clone, Debug)]
pub struct PackageSource<'a> {
    pub package_name: CompiledPackageName<'a>,
    pub source_dir: Cow<'a, Path>,
}

impl<'a> PackageSource<'a> {
    pub fn to_arg(&self) -> String {
        format!("{}:{}", self.package_name, self.source_dir.display())
    }
}

#[derive(Clone, Debug)]
pub struct VirtualPackageImplementation<'a> {
    pub mi_path: Cow<'a, Path>,
    pub package_name: &'a PackageFQN,
    pub package_path: Cow<'a, Path>,
}

#[derive(Clone, Debug)]
pub struct CompilationFlags {
    /// Disable optimization (adds -O0)
    pub no_opt: bool,
    /// Include debug symbols (adds -g)
    pub symbols: bool,
    pub source_map: bool,
    pub enable_coverage: bool,
    pub self_coverage: bool,
    pub enable_value_tracing: bool,
}

/// Configuration for either warning or alert
#[derive(Clone, Debug, Default)]
#[allow(unused)]
pub enum WarnAlertConfig<'a> {
    #[default]
    Default,
    List(Cow<'a, str>),
    DenyAll,
    AllowAll,
}

pub trait CmdlineAbstraction {
    fn to_args(&self, args: &mut Vec<String>);

    fn build_command(&self, executable: impl Into<String>) -> Vec<String> {
        let mut args = vec![executable.into()];
        self.to_args(&mut args);
        args
    }
}

#[allow(unused)]
pub(crate) const MOONC_REGULAR_WARNING_SET: &str = "+a-31-32";
#[allow(unused)]
pub(crate) const MOONC_REGULAR_ALERT_SET: &str = "+all-raise-throw-unsafe+deprecated";

pub(crate) const MOONC_DENY_WARNING_SET: &str = "@a-31-32";
pub(crate) const MOONC_DENY_ALERT_SET: &str = "@all-raise-throw-unsafe+deprecated";
pub(crate) const MOONC_ALLOW_WARNING_SET: &str = "-a";
pub(crate) const MOONC_ALLOW_ALERT_SET: &str = "-all";
