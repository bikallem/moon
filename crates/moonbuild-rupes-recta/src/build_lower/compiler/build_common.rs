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

//! Common types and functionality shared between compiler command abstractions

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use moonutil::common::TargetBackend;

use crate::build_lower::compiler::{
    CompiledPackageName, ErrorFormat, MOONC_ALLOW_ALERT_SET, MOONC_ALLOW_WARNING_SET,
    MOONC_DENY_ALERT_SET, MOONC_DENY_WARNING_SET, MiDependency, VirtualPackageImplementation,
    WarnAlertConfig,
};
use crate::model::TargetKind;

/// Required (non-default) fields shared between different build-like commands of `moonc`
#[derive(Debug)]
pub struct BuildCommonInput<'a> {
    /// Regular input files, including sources and mbt.md files
    pub mbt_sources: &'a [PathBuf],
    /// Sources that only needs doctest extraction
    pub doctest_only_sources: &'a [PathBuf],
    /// MI deps required to resolve interfaces
    pub mi_deps: &'a [MiDependency<'a>],
    /// The name of the current package
    pub package_name: CompiledPackageName<'a>,
    /// The source directory of the current package
    pub package_source: Cow<'a, Path>,

    // Target configuration
    /// Target backend to compile for
    pub target_backend: TargetBackend,
    /// Target kind (source/test/etc.)
    pub target_kind: TargetKind,
}

impl<'a> BuildCommonInput<'a> {
    /// Construct the required part from its required params
    pub fn new(
        mbt_sources: &'a [PathBuf],
        doctest_only_sources: &'a [PathBuf],
        mi_deps: &'a [MiDependency<'a>],
        package_name: CompiledPackageName<'a>,
        package_source: impl Into<Cow<'a, Path>>,
        target_backend: TargetBackend,
        target_kind: TargetKind,
    ) -> Self {
        Self {
            mbt_sources,
            doctest_only_sources,
            mi_deps,
            package_name,
            package_source: package_source.into(),
            target_backend,
            target_kind,
        }
    }

    /// Add MBT source files as arguments
    pub fn add_mbt_sources(&self, args: &mut Vec<String>) {
        for mbt_file in self.mbt_sources {
            args.push(mbt_file.display().to_string());
        }
    }

    /// Add doctest-only MBT sources as -doctest-only pairs
    pub fn add_doctest_only_sources(&self, args: &mut Vec<String>) {
        for src in self.doctest_only_sources {
            args.extend(["-doctest-only".to_string(), src.display().to_string()]);
        }
    }

    /// Add MI dependencies arguments
    pub fn add_mi_dependencies(&self, args: &mut Vec<String>) {
        for mi_dep in self.mi_deps {
            args.extend(["-i".to_string(), mi_dep.to_alias_arg()]);
        }
    }

    /// Add package configuration arguments
    pub fn add_package_config(&self, args: &mut Vec<String>) {
        args.extend(["-pkg".to_string(), self.package_name.to_string()]);
    }

    /// Add package source definition arguments
    pub fn add_package_sources(&self, args: &mut Vec<String>) {
        args.extend([
            "-pkg-sources".to_string(),
            format!("{}:{}", self.package_name, self.package_source.display()),
        ]);
    }

    /// Add target backend arguments
    pub fn add_target_backend(&self, args: &mut Vec<String>) {
        args.extend([
            "-target".to_string(),
            self.target_backend.to_flag().to_string(),
        ]);
    }

    /// Add white/black box test arguments
    pub fn add_test_args(&self, args: &mut Vec<String>) {
        match self.target_kind {
            TargetKind::WhiteboxTest => args.push("-whitebox-test".into()),
            TargetKind::BlackboxTest => {
                args.push("-blackbox-test".into());
                args.push("-include-doctests".into());
            }
            TargetKind::Source | TargetKind::InlineTest | TargetKind::SubPackage => {}
        }
    }

    pub fn add_test_mode_args(&self, args: &mut Vec<String>) {
        if self.target_kind.is_test() {
            args.push("-test-mode".into())
        }
    }

    /// Emit -include-doctests for blackbox
    pub fn add_include_doctests_if_blackbox(&self, args: &mut Vec<String>) {
        if matches!(self.target_kind, TargetKind::BlackboxTest) {
            args.push("-include-doctests".to_string());
        }
    }

    /// Emit test kind flags
    pub fn add_test_kind_flags(&self, args: &mut Vec<String>) {
        match self.target_kind {
            TargetKind::WhiteboxTest => args.push("-whitebox-test".to_string()),
            TargetKind::BlackboxTest => args.push("-blackbox-test".to_string()),
            _ => {}
        }
    }
}

/// Defaultable fields shared between different build-like commands of `moonc`
#[derive(Debug)]
pub struct BuildCommonConfig<'a> {
    // Basic command structure
    pub error_format: ErrorFormat,

    // Warning and alert configuration
    pub deny_warn: bool,
    pub warn_config: WarnAlertConfig<'a>,
    pub alert_config: WarnAlertConfig<'a>,

    // Input files

    // Package configuration
    pub is_main: bool,

    // Standard library
    /// Pass [None] for no_std
    pub stdlib_core_file: Option<Cow<'a, Path>>,
    /// Module directory (parent of moon.mod.json)
    pub workspace_root: Option<Cow<'a, Path>>,

    // Virtual package handling
    // FIXME: better abstraction
    pub check_mi: Option<Cow<'a, Path>>,
    pub virtual_implementation: Option<VirtualPackageImplementation<'a>>,

    // Optional patch file
    pub patch_file: Option<Cow<'a, Path>>,

    // Emit -no-mi if true
    pub no_mi: bool,
}

impl<'a> Default for BuildCommonConfig<'a> {
    fn default() -> Self {
        Self {
            error_format: ErrorFormat::Regular,
            deny_warn: false,
            warn_config: WarnAlertConfig::Default,
            alert_config: WarnAlertConfig::Default,
            is_main: false,
            stdlib_core_file: None,
            workspace_root: None,
            check_mi: None,
            virtual_implementation: None,
            patch_file: None,
            no_mi: false,
        }
    }
}

impl<'a> BuildCommonConfig<'a> {
    /// Add error format arguments
    pub fn add_error_format(&self, args: &mut Vec<String>) {
        if matches!(self.error_format, ErrorFormat::Json) {
            args.extend(["-error-format".to_string(), "json".to_string()]);
        }
    }

    /// Add custom warning/alert list arguments
    pub fn add_custom_warn_alert_lists(&self, args: &mut Vec<String>) {
        if let WarnAlertConfig::List(warn_list) = &self.warn_config {
            args.extend(["-w".to_string(), warn_list.to_string()]);
        }
        if let WarnAlertConfig::List(alert_list) = &self.alert_config {
            args.extend(["-alert".to_string(), alert_list.to_string()]);
        }
    }

    /// Add is-main flag if applicable
    pub fn add_is_main(&self, args: &mut Vec<String>) {
        if self.is_main {
            args.push("-is-main".to_string());
        }
    }

    /// Add standard library path arguments
    pub fn add_stdlib_path(&self, args: &mut Vec<String>) {
        if let Some(stdlib_path) = &self.stdlib_core_file {
            args.extend(["-std-path".to_string(), stdlib_path.display().to_string()]);
        }
    }

    /// Add virtual package check arguments
    pub fn add_virtual_package_check(&self, args: &mut Vec<String>) {
        if let Some(check_mi_path) = &self.check_mi {
            args.extend(["-check-mi".to_string(), check_mi_path.display().to_string()]);
        }
    }

    /// Add warning/alert deny all arguments (combined)
    pub fn add_deny_all(&self, args: &mut Vec<String>) {
        if self.deny_warn {
            args.extend([
                "-w".to_string(),
                MOONC_DENY_WARNING_SET.to_string(),
                "-alert".to_string(),
                MOONC_DENY_ALERT_SET.to_string(),
            ]);
        }
    }

    /// Add warning/alert allow all arguments
    pub fn add_warn_alert_allow_all(&self, args: &mut Vec<String>) {
        if matches!(self.warn_config, WarnAlertConfig::AllowAll) {
            args.extend(["-w".to_string(), MOONC_ALLOW_WARNING_SET.into()]);
        }
        if matches!(self.alert_config, WarnAlertConfig::AllowAll) {
            args.extend(["-alert".to_string(), MOONC_ALLOW_ALERT_SET.into()]);
        }
    }

    /// Add virtual package implementation arguments (with different behavior for check vs build-package)
    pub fn add_virtual_package_implementation_check(&self, args: &mut Vec<String>) {
        if let Some(impl_virtual) = &self.virtual_implementation {
            args.extend([
                "-check-mi".to_string(),
                impl_virtual.mi_path.display().to_string(),
                "-pkg-sources".to_string(),
                format!(
                    "{}:{}",
                    impl_virtual.package_name,
                    impl_virtual.package_path.display()
                ),
            ]);
        }
    }

    /// Add virtual package implementation arguments for build-package.
    /// Note: does NOT emit -no-mi. The caller must control `-no-mi` via
    /// [BuildCommonConfig::no_mi] and let [BuildCommonConfig::add_no_mi] handle
    /// emission.
    pub fn add_virtual_package_implementation_build(&self, args: &mut Vec<String>) {
        if let Some(impl_virtual) = &self.virtual_implementation {
            args.extend([
                "-check-mi".to_string(),
                impl_virtual.mi_path.display().to_string(),
                "-impl-virtual".to_string(),
                "-pkg-sources".to_string(),
                format!(
                    "{}:{}",
                    impl_virtual.package_name,
                    impl_virtual.package_path.display()
                ),
            ]);
        }
    }

    /// Add workspace root flag (module directory)
    pub fn add_workspace_root(&self, args: &mut Vec<String>) {
        // Note: -workspace-path is not supported for link-core yet. Builders that
        // emit link-core must not include it. The specific builders decide whether
        // to add this flag; this common helper is only used by check/build-package.
        if let Some(ws) = &self.workspace_root {
            args.extend(["-workspace-path".to_string(), ws.display().to_string()]);
        }
    }

    /// Emit -patch-file
    pub fn add_patch_file_moonc(&self, args: &mut Vec<String>) {
        if let Some(patch) = &self.patch_file {
            args.extend(["-patch-file".to_string(), patch.display().to_string()]);
        }
    }

    /// Emit -no-mi if enabled
    pub fn add_no_mi(&self, args: &mut Vec<String>) {
        if self.no_mi {
            args.push("-no-mi".to_string());
        }
    }
}
