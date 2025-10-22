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

//! Lowering implementation for auxiliary build steps.

use moonutil::{
    common::DriverKind,
    compiler_flags::{
        CC, CCConfigBuilder, OptLevel as CCOptLevel, OutputType as CCOutputType,
        make_cc_command_pure, resolve_cc,
    },
    mooncakes::{ModuleId, ModuleSourceKind},
};
use tracing::{Level, instrument};

use crate::{
    build_lower::compiler::{CmdlineAbstraction, MoondocCommand, Mooninfo},
    build_plan::BuildTargetInfo,
    model::{BuildPlanNode, BuildTarget, PackageId, TargetKind},
};

use super::{BuildCommand, compiler};

impl<'a> super::BuildPlanLowerContext<'a> {
    #[instrument(level = Level::DEBUG, skip(self, info))]
    pub(super) fn lower_gen_test_driver(
        &mut self,
        _node: BuildPlanNode,
        target: BuildTarget,
        info: &BuildTargetInfo,
    ) -> BuildCommand {
        let package = self.get_package(target);
        let output_driver =
            self.layout
                .generated_test_driver(self.packages, &target, self.opt.target_backend);
        let output_metadata = self.layout.generated_test_driver_metadata(
            self.packages,
            &target,
            self.opt.target_backend,
        );
        let driver_kind = match target.kind {
            TargetKind::Source => panic!("Source package cannot be a test driver"),
            TargetKind::WhiteboxTest => DriverKind::Whitebox,
            TargetKind::BlackboxTest => DriverKind::Blackbox,
            TargetKind::InlineTest => DriverKind::Internal,
            TargetKind::SubPackage => panic!("Sub-package cannot be a test driver"),
        };
        let pkg_full_name = package.fqn.to_string();
        let files_vec = if target.kind == TargetKind::WhiteboxTest {
            info.whitebox_files.clone()
        } else {
            info.files().map(|x| x.to_owned()).collect::<Vec<_>>()
        };
        let patch_file = info.patch_file.as_deref().map(|x| x.into());

        let cmd = compiler::MoonGenTestDriver {
            files: &files_vec,
            doctest_only_files: &info.doctest_files,
            output_driver: output_driver.into(),
            output_metadata: output_metadata.into(),
            bench: false, // TODO
            enable_coverage: self.opt.enable_coverage,
            coverage_package_override: None, // TODO,
            driver_kind,
            target_backend: self.opt.target_backend,
            patch_file,
            pkg_name: &pkg_full_name,
        };

        BuildCommand {
            commandline: cmd.build_command("moon"),
            extra_inputs: files_vec,
        }
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub(super) fn lower_bundle(
        &mut self,
        node: BuildPlanNode,
        module_id: ModuleId,
    ) -> BuildCommand {
        let module = self.modules.mod_name_from_id(module_id);
        let output = self
            .layout
            .bundle_result_path(self.opt.target_backend, module.name());

        let mut inputs = vec![];
        for dep in self.build_plan.dependency_nodes(node) {
            let BuildPlanNode::BuildCore(package) = dep else {
                panic!("Bundle node can only depend on BuildCore nodes");
            };
            inputs.push(self.layout.core_of_build_target(
                self.packages,
                &package,
                self.opt.target_backend,
            ));
        }
        inputs.sort();

        let cmd = compiler::MooncBundleCore::new(&inputs, output);

        BuildCommand {
            extra_inputs: vec![],
            commandline: cmd.build_command("moonc"),
        }
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub(super) fn lower_compile_runtime(&mut self) -> BuildCommand {
        let artifact_path = self
            .layout
            .runtime_output_path(self.opt.target_backend, self.opt.os);

        // TODO: this part might need more simplification?
        let runtime_c_path = self.opt.runtime_dot_c_path.clone();
        let cc_cmd = make_cc_command_pure::<&'static str>(
            resolve_cc(CC::default(), None),
            CCConfigBuilder::default()
                .no_sys_header(true)
                .output_ty(CCOutputType::Object)
                .opt_level(CCOptLevel::Speed)
                .debug_info(true)
                // always link moonbitrun in this mode
                .link_moonbitrun(true)
                .define_use_shared_runtime_macro(false)
                .build()
                .expect("Failed to build CC configuration for runtime"),
            &[],
            [runtime_c_path.display().to_string()],
            &self.opt.target_dir_root.display().to_string(),
            &artifact_path.display().to_string(),
            &self.opt.compiler_paths,
        );

        BuildCommand {
            extra_inputs: vec![runtime_c_path],
            commandline: cc_cmd,
        }
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub(super) fn lower_generate_mbti(&mut self, target: BuildTarget) -> BuildCommand {
        let input = self
            .layout
            .mi_of_build_target(self.packages, &target, self.opt.target_backend);
        let pkg = self.packages.get_package(target.package);
        let output = self.layout.generated_mbti_path(&pkg.root_path);

        let cmd = Mooninfo {
            mi_in: input.into(),
            out: output.into(),
            no_alias: self.opt.info_no_alias,
        };

        BuildCommand {
            extra_inputs: vec![],
            commandline: cmd.build_command("mooninfo"),
        }
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub(super) fn lower_build_docs(&self) -> BuildCommand {
        // TODO: How to enforce the `packages.json` dependency is generated
        // up-to-date before the command is executed?
        //
        // If we forgot to generate anything at all, we can get a complaint from
        // n2 for the file doesn't exist and nobody can create it, but if we
        // have a stale file, we currently have to rely on ourselves.
        //
        // Currently, moondoc only support a single module in scope, so we
        // have these constraints
        let main_module = self
            .opt
            .main_module
            .as_ref()
            .expect("Currently only one module in the workspace is supported.");
        let path = match main_module.source() {
            ModuleSourceKind::Local(p) => p,
            ModuleSourceKind::Registry(_)
            | ModuleSourceKind::Git(_)
            | ModuleSourceKind::Stdlib(_) => {
                panic!("Remote modules for docs are not supported")
            }
        };

        let packages_json = self.layout.packages_json_path();
        let cmd = MoondocCommand::new(
            path,
            self.layout.doc_dir(),
            self.opt.stdlib_path.as_ref(),
            &packages_json,
            self.opt.docs_serve,
        );

        BuildCommand {
            commandline: cmd.build_command("moondoc"),
            extra_inputs: vec![packages_json],
        }
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub(super) fn lower_run_prebuild(&self, pkg: PackageId, idx: u32) -> BuildCommand {
        let info = self
            .build_plan
            .get_prebuild_info(pkg, idx)
            .expect("Prebuild info should be populated before lowering run prebuild");

        // Note: we are tracking dependencies between prebuild commands via n2.
        // Ideally we can do this ourselves, but n2 does it anyway so we don't bother.

        BuildCommand {
            commandline: vec!["sh".into(), "-c".into(), info.command.clone()],
            extra_inputs: info.resolved_inputs.clone(),
        }
    }
}
