use std::path::PathBuf;

use crate::shared::errors::AppError;

#[derive(Debug)]
pub(crate) struct PreparedFolderCreate {
    pub(crate) stage: PathBuf,
    pub(crate) target: PathBuf,
    parent_proof: crate::platform::fs::file_utils::FilesystemIdentityProof,
    pub(crate) namespace_proof: Option<crate::platform::fs::file_utils::FilesystemNamespaceProof>,
    stage_proof: std::sync::OnceLock<crate::platform::fs::file_utils::FilesystemIdentityProof>,
}

impl PreparedFolderCreate {
    pub(crate) fn new(stage: PathBuf, target: PathBuf) -> Result<Self, AppError> {
        let parent = stage
            .parent()
            .ok_or_else(|| AppError::Validation("Folder creation parent is missing".to_string()))?;
        if target.parent() != Some(parent) {
            return Err(AppError::Validation(
                "Folder staging must share its target parent".to_string(),
            ));
        }
        Ok(Self {
            parent_proof: crate::platform::fs::file_utils::FilesystemIdentityProof::capture(
                parent,
            )?,
            stage,
            target,
            namespace_proof: None,
            stage_proof: std::sync::OnceLock::new(),
        })
    }

    pub(crate) fn prepare(&self) -> Result<(), AppError> {
        let parent = crate::platform::fs::rename::PinnedDirectory::open(
            self.stage.parent().expect("prepared parent"),
        )?;
        if parent.identity() != self.parent_proof.identity() {
            return Err(AppError::Security(
                "Folder creation parent was replaced before staging".to_string(),
            ));
        }
        self.validate_parent()?;
        let _ancestry = self.pin_namespace()?;
        let child = parent.create_child(self.stage.file_name().expect("prepared stage name"))?;
        let proof = crate::platform::fs::file_utils::FilesystemIdentityProof::capture(&self.stage)?;
        if child.identity() != proof.identity() {
            return Err(AppError::Security(
                "Folder staging changed before ownership capture".to_string(),
            ));
        }
        self.stage_proof.set(proof).map_err(|_| {
            AppError::Internal("Folder creation staging was already prepared".to_string())
        })
    }

    pub(crate) fn journal_step(
        &self,
    ) -> Result<crate::modules::mutation::api::PlannedStep, AppError> {
        let proof = self.owned_stage()?;
        Ok(crate::modules::mutation::api::PlannedStep::rename(
            0,
            self.stage.clone(),
            self.target.clone(),
        )
        .with_expected_identity(Some(proof.identity().to_string()))
        .with_source_root_identity(self.parent_proof.identity().to_string()))
    }

    pub(crate) fn promote(&self) -> Result<(), AppError> {
        let parent = crate::platform::fs::rename::PinnedDirectory::open(
            self.stage.parent().expect("prepared parent"),
        )?;
        if parent.identity() != self.parent_proof.identity() {
            return Err(AppError::Security(
                "Folder creation parent was replaced".to_string(),
            ));
        }
        self.validate_parent()?;
        let _ancestry = self.pin_namespace()?;
        #[cfg(not(windows))]
        {
            self.owned_stage()?.validate(&self.stage)?;
            return crate::platform::fs::rename::rename_no_replace(&self.stage, &self.target)
                .map_err(AppError::from);
        }
        #[cfg(windows)]
        {
            let source =
                crate::platform::fs::rename::PinnedDirectory::open_for_mutation(&self.stage)?;
            if source.identity() != self.owned_stage()?.identity() {
                return Err(AppError::Security(
                    "Folder creation staging was replaced".to_string(),
                ));
            }
            source.rename_to(
                &parent,
                self.target.file_name().expect("prepared target name"),
            )?;
            Ok(())
        }
    }

    pub(crate) fn rollback(&self) -> Result<(), AppError> {
        let parent = crate::platform::fs::rename::PinnedDirectory::open(
            self.stage.parent().expect("prepared parent"),
        )?;
        if parent.identity() != self.parent_proof.identity() {
            return Err(AppError::Security(
                "Folder creation rollback parent was replaced".to_string(),
            ));
        }
        self.validate_parent()?;
        let _ancestry = self.pin_namespace()?;
        if !self.stage.exists() && !self.target.exists() {
            return Ok(());
        }
        let proof = self.owned_stage()?;
        let path = if self.stage.exists() {
            &self.stage
        } else {
            &self.target
        };
        #[cfg(not(windows))]
        {
            proof.validate(path)?;
            return std::fs::remove_dir(path).map_err(AppError::from);
        }
        #[cfg(windows)]
        {
            let owned = crate::platform::fs::rename::PinnedDirectory::open_for_mutation(path)?;
            if owned.identity() != proof.identity() {
                return Err(AppError::Security(
                    "Folder creation rollback directory was replaced".to_string(),
                ));
            }
            owned.remove_empty(&parent).map_err(AppError::from)
        }
    }

    pub(crate) fn owned_stage(
        &self,
    ) -> Result<&crate::platform::fs::file_utils::FilesystemIdentityProof, AppError> {
        self.stage_proof.get().ok_or_else(|| {
            AppError::Io(
                "Folder creation has no owned staging identity; retained paths require review"
                    .to_string(),
            )
        })
    }

    pub(crate) fn pin_target(
        &self,
    ) -> Result<Vec<crate::platform::fs::rename::PinnedDirectory>, AppError> {
        self.validate_parent()?;
        let mut pins = self.pin_namespace()?;
        let target = crate::platform::fs::rename::PinnedDirectory::open(&self.target)?;
        if target.identity() != self.owned_stage()?.identity() {
            return Err(AppError::Security(
                "Folder creation target was replaced".to_string(),
            ));
        }
        pins.push(target);
        Ok(pins)
    }

    fn pin_namespace(&self) -> Result<Vec<crate::platform::fs::rename::PinnedDirectory>, AppError> {
        match &self.namespace_proof {
            Some(proof) => proof.pin_paths(&[self.stage.clone(), self.target.clone()]),
            None => Ok(Vec::new()),
        }
    }

    fn validate_parent(&self) -> Result<(), AppError> {
        let parent = self
            .stage
            .parent()
            .ok_or_else(|| AppError::Validation("Folder creation parent is missing".to_string()))?;
        self.parent_proof.validate(parent)?;
        if let Some(proof) = &self.namespace_proof {
            proof.validate_paths(&[self.stage.clone(), self.target.clone()])?;
        }
        Ok(())
    }
}
