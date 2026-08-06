use std::{
    any::{self, TypeId, type_name},
    collections::{BTreeMap, hash_map},
    path::PathBuf,
    rc::Rc,
};

use anyhow::{Context, Result};
use futures::{StreamExt, channel::mpsc, future::LocalBoxFuture};
use gpui::{App, AsyncApp, BorrowAppContext, FontFallbacks, Task, px};
use settings_content::{
    ScrollbarSettings, SettingsContent, UserSettingsContent, terminal::PathHyperlinkRegex,
};
use utils::collections::TypeIdHashMap;

use crate::{
    AnySettingValue, RegisteredSetting, SettingValue, Settings, SettingsFile, SettingsParseResult,
    parse_json_with_comments,
};

inventory::collect!(RegisteredSetting);

pub struct SettingsStore {
    setting_values: TypeIdHashMap<Box<dyn AnySettingValue>>,
    default_settings: Rc<SettingsContent>,
    user_settings: Option<UserSettingsContent>,

    // extension_settings: Option<Box<SettingsContent>>,
    merged_settings: Rc<SettingsContent>,

    last_user_settings_content: Option<String>,
    // last_global_settings_content: Option<String>,
    _settings_files_watcher: Option<Task<()>>,
    _setting_file_updates: Task<()>,
    setting_file_updates_tx: mpsc::UnboundedSender<
        Box<dyn FnOnce(AsyncApp) -> LocalBoxFuture<'static, anyhow::Result<()>>>,
    >,
    file_errors: BTreeMap<SettingsFile, SettingsParseResult>,
}

impl SettingsStore {
    pub fn new(cx: &mut App, default_settings: &str) -> Self {
        Self::new_with_semantic_tokens(cx, default_settings)
    }

    pub fn new_with_semantic_tokens(cx: &mut App, default_settings: &str) -> Self {
        let default_settings = Self::parse_default_settings(default_settings).unwrap();
        Self::from_settings_content(cx, default_settings)
    }

    fn from_settings_content(cx: &mut App, default_settings: SettingsContent) -> Self {
        let (setting_file_updates_tx, mut setting_file_updates_rx) = mpsc::unbounded();

        let default_settings: Rc<SettingsContent> = default_settings.into();
        let mut this = Self {
            setting_values: Default::default(),
            default_settings: default_settings.clone(),
            user_settings: None,
            // extension_settings: None,
            // language_semantic_token_rules: HashMap::default(),
            merged_settings: default_settings,
            last_user_settings_content: None,
            // last_global_settings_content: None,
            // local_settings: BTreeMap::default(),
            // editorconfig_store: cx.new(|_| EditorconfigStore::default()),
            _settings_files_watcher: None,
            setting_file_updates_tx,
            _setting_file_updates: cx.spawn(async move |cx| {
                while let Some(setting_file_update) = setting_file_updates_rx.next().await {
                    // (setting_file_update)(cx.clone()).await;
                }
            }),
            file_errors: BTreeMap::default(),
        };

        this.load_settings_types();

        this
    }

    pub fn update<C, R>(cx: &mut C, f: impl FnOnce(&mut Self, &mut C) -> R) -> R
    where
        C: BorrowAppContext,
    {
        cx.update_global(f)
    }

    /// Add a new type of setting to the store.
    /// 将设置注册到全局的配置中心
    pub fn register_setting<T: Settings>(&mut self) {
        self.register_setting_internal(&RegisteredSetting {
            settings_value: || Box::new(SettingValue::<T> { global_value: None }),
            from_settings: |content| Box::new(T::from_settings(content)),
            id: || TypeId::of::<T>(),
        });
    }

    // 把通过注册的设置类型加载到全局配置中心 例如：ThemeSettings,  TerminalSettings
    fn load_settings_types(&mut self) {
        for registered_setting in inventory::iter::<RegisteredSetting>() {
            self.register_setting_internal(registered_setting);
        }
    }

    fn register_setting_internal(&mut self, registered_setting: &RegisteredSetting) {
        let entry = self.setting_values.entry((registered_setting.id)());

        if matches!(entry, hash_map::Entry::Occupied(_)) {
            return;
        }

        let setting_value = entry.or_insert((registered_setting.settings_value)());
        let value = (registered_setting.from_settings)(&self.merged_settings);
        setting_value.set_global_value(value);
    }

    pub fn merged_settings(&self) -> &SettingsContent {
        &self.merged_settings
    }

    /// Get the value of a setting.
    ///
    /// Panics if the given setting type has not been registered, or if there is no
    /// value for this setting.
    pub fn get<T: Settings>(&self, path: Option<String>) -> &T {
        // self.setting_values
        //     .get(&TypeId::of::<T>())
        //     .unwrap_or_else(|| panic!("unregistered setting type {}", type_name::<T>()))
        //     .value_for_path(path)
        //     .downcast_ref::<T>()
        //     .expect("no default value for setting type")
        todo!()
    }

    /// Get the value of a setting.
    ///
    /// Does not panic
    pub fn try_get<T: Settings>(&self, path: Option<String>) -> Option<&T> {
        // self.setting_values
        //     .get(&TypeId::of::<T>())
        //     .map(|value| value.value_for_path(path))
        //     .and_then(|value| value.downcast_ref::<T>())
        todo!()
    }

    // /// Get all values from project specific settings
    // pub fn get_all_locals<T: Settings>(&self) -> Vec<(WorktreeId, Arc<RelPath>, &T)> {
    //     self.setting_values
    //         .get(&TypeId::of::<T>())
    //         .unwrap_or_else(|| panic!("unregistered setting type {}", type_name::<T>()))
    //         .all_local_values()
    //         .into_iter()
    //         .map(|(id, path, any)| {
    //             (
    //                 id,
    //                 path,
    //                 any.downcast_ref::<T>()
    //                     .expect("wrong value type for setting"),
    //             )
    //         })
    //         .collect()
    // }

    /// Override the global value for a setting.
    ///
    /// The given value will be overwritten if the user settings file changes.
    pub fn override_global<T: Settings>(&mut self, value: T) {
        self.setting_values
            .get_mut(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("unregistered setting type {}", type_name::<T>()))
            .set_global_value(Box::new(value))
    }

    /// Get the user's settings content.
    ///
    /// For user-facing functionality use the typed setting interface.
    /// (e.g. ProjectSettings::get_global(cx))
    pub fn raw_user_settings(&self) -> Option<&UserSettingsContent> {
        self.user_settings.as_ref()
    }

    /// Get the default settings content as a raw JSON value.
    pub fn raw_default_settings(&self) -> &SettingsContent {
        &self.default_settings
    }

    /// Get the configured settings profile names.
    pub fn configured_settings_profiles(&self) -> impl Iterator<Item = &str> {
        self.user_settings
            .iter()
            .flat_map(|settings| settings.profiles.keys().map(|k| k.as_str()))
    }

    pub async fn load_settings(path: &PathBuf) -> Result<String> {
        std::fs::read_to_string(path).context("Failed to read settings file")
    }

    // fn update_settings_file_inner(
    //     &self,
    //     fs: Arc<dyn Fs>,
    //     update: impl 'static + Send + FnOnce(String, AsyncApp) -> Result<String>,
    // ) -> oneshot::Receiver<Result<()>> {
    //     let (tx, rx) = oneshot::channel::<Result<()>>();
    //     self.setting_file_updates_tx
    //         .unbounded_send(Box::new(move |cx: AsyncApp| {
    //             async move {
    //                 let res = async move {
    //                     let old_text = Self::load_settings(&fs).await?;
    //                     let new_text = update(old_text, cx.clone())?;

    //                     let settings_path = paths::settings_file().as_path();
    //                     if fs.is_file(settings_path).await {
    //                         let resolved_path =
    //                             fs.canonicalize(settings_path).await.with_context(|| {
    //                                 format!(
    //                                     "Failed to canonicalize settings path {:?}",
    //                                     settings_path
    //                                 )
    //                             })?;

    //                         fs.atomic_write(resolved_path.clone(), new_text.clone())
    //                             .await
    //                             .with_context(|| {
    //                                 format!("Failed to write settings to file {:?}", resolved_path)
    //                             })?;
    //                     } else {
    //                         fs.atomic_write(settings_path.to_path_buf(), new_text.clone())
    //                             .await
    //                             .with_context(|| {
    //                                 format!("Failed to write settings to file {:?}", settings_path)
    //                             })?;
    //                     }

    //                     cx.update_global(|store: &mut SettingsStore, cx| {
    //                         store.set_user_settings(&new_text, cx).result().map(|_| ())
    //                     })
    //                 }
    //                 .await;

    //                 let new_res = match &res {
    //                     Ok(_) => anyhow::Ok(()),
    //                     Err(e) => Err(anyhow::anyhow!("{:?}", e)),
    //                 };

    //                 _ = tx.send(new_res);
    //                 res
    //             }
    //             .boxed_local()
    //         }))
    //         .map_err(|err| anyhow::format_err!("Failed to update settings file: {}", err))
    //         .log_with_level(log::Level::Warn);
    //     return rx;
    // }

    // pub fn update_settings_file(
    //     &self,
    //     fs: Arc<dyn Fs>,
    //     update: impl 'static + Send + FnOnce(&mut SettingsContent, &App),
    // ) {
    //     _ = self.update_settings_file_with_completion(fs, update);
    // }

    // pub fn update_settings_file_with_completion(
    //     &self,
    //     fs: Arc<dyn Fs>,
    //     update: impl 'static + Send + FnOnce(&mut SettingsContent, &App),
    // ) -> oneshot::Receiver<Result<()>> {
    //     self.update_settings_file_inner(fs, move |old_text: String, cx: AsyncApp| {
    //         cx.read_global(|store: &SettingsStore, cx| {
    //             store.new_text_for_update(old_text, |content| update(content, cx))
    //         })
    //     })
    // }

    // pub fn get_all_files(&self) -> Vec<SettingsFile> {
    //     let mut files = Vec::from_iter(
    //         self.local_settings
    //             .keys()
    //             // rev because these are sorted by path, so highest precedence is last
    //             .rev()
    //             .cloned()
    //             .map(SettingsFile::Project),
    //     );

    //     if self.server_settings.is_some() {
    //         files.push(SettingsFile::Server);
    //     }
    //     // ignoring profiles
    //     // ignoring os profiles
    //     // ignoring release channel profiles
    //     // ignoring global
    //     // ignoring extension

    //     if self.user_settings.is_some() {
    //         files.push(SettingsFile::User);
    //     }
    //     files.push(SettingsFile::Default);
    //     files
    // }

    pub fn get_content_for_file(&self, file: SettingsFile) -> Option<&SettingsContent> {
        match file {
            SettingsFile::User => self
                .user_settings
                .as_ref()
                .map(|settings| settings.content.as_ref()),
            SettingsFile::Default => Some(self.default_settings.as_ref()),
            // SettingsFile::Global => self.global_settings.as_deref(),
        }
    }

    // pub fn get_overrides_for_field<T>(
    //     &self,
    //     target_file: SettingsFile,
    //     get: fn(&SettingsContent) -> &Option<T>,
    // ) -> Vec<SettingsFile> {
    //     let all_files = self.get_all_files();
    //     let mut found_file = false;
    //     let mut overrides = Vec::new();

    //     for file in all_files.into_iter().rev() {
    //         if !found_file {
    //             found_file = file == target_file;
    //             continue;
    //         }

    //         if let SettingsFile::Project((wt_id, ref path)) = file
    //             && let SettingsFile::Project((target_wt_id, ref target_path)) = target_file
    //             && (wt_id != target_wt_id || !target_path.starts_with(path))
    //         {
    //             // if requesting value from a local file, don't return values from local files in different worktrees
    //             continue;
    //         }

    //         let Some(content) = self.get_content_for_file(file.clone()) else {
    //             continue;
    //         };
    //         if get(content).is_some() {
    //             overrides.push(file);
    //         }
    //     }

    //     overrides
    // }

    /// Checks the given file, and files that the passed file overrides for the given field.
    /// Returns the first file found that contains the value.
    /// The value will only be None if no file contains the value.
    /// I.e. if no file contains the value, returns `(File::Default, None)`
    // pub fn get_value_from_file<'a, T: 'a>(
    //     &'a self,
    //     target_file: SettingsFile,
    //     pick: fn(&'a SettingsContent) -> Option<T>,
    // ) -> (SettingsFile, Option<T>) {
    //     self.get_value_from_file_inner(target_file, pick, true)
    // }

    // /// Same as `Self::get_value_from_file` except that it does not include the current file.
    // /// Therefore it returns the value that was potentially overloaded by the target file.
    // pub fn get_value_up_to_file<'a, T: 'a>(
    //     &'a self,
    //     target_file: SettingsFile,
    //     pick: fn(&'a SettingsContent) -> Option<T>,
    // ) -> (SettingsFile, Option<T>) {
    //     self.get_value_from_file_inner(target_file, pick, false)
    // }

    // fn get_value_from_file_inner<'a, T: 'a>(
    //     &'a self,
    //     target_file: SettingsFile,
    //     pick: fn(&'a SettingsContent) -> Option<T>,
    //     include_target_file: bool,
    // ) -> (SettingsFile, Option<T>) {
    //     // todo(settings_ui): Add a metadata field for overriding the "overrides" tag, for contextually different settings
    //     //  e.g. disable AI isn't overridden, or a vec that gets extended instead or some such

    //     // todo(settings_ui) cache all files
    //     let all_files = self.get_all_files();
    //     let mut found_file = false;

    //     for file in all_files.into_iter() {
    //         if !found_file && file != SettingsFile::Default {
    //             if file != target_file {
    //                 continue;
    //             }
    //             found_file = true;
    //             if !include_target_file {
    //                 continue;
    //             }
    //         }

    //         if let SettingsFile::Project((worktree_id, ref path)) = file
    //             && let SettingsFile::Project((target_worktree_id, ref target_path)) = target_file
    //             && (worktree_id != target_worktree_id || !target_path.starts_with(&path))
    //         {
    //             // if requesting value from a local file, don't return values from local files in different worktrees
    //             continue;
    //         }

    //         let Some(content) = self.get_content_for_file(file.clone()) else {
    //             continue;
    //         };
    //         if let Some(value) = pick(content) {
    //             return (file, Some(value));
    //         }
    //     }

    //     (SettingsFile::Default, None)
    // }

    pub fn error_for_file(&self, file: SettingsFile) -> Option<SettingsParseResult> {
        self.file_errors
            .get(&file)
            .filter(|parse_result| parse_result.requires_user_action())
            .cloned()
    }
}

impl SettingsStore {
    /// Updates the value of a setting in a JSON file, returning the new text
    /// for that JSON file.
    // pub fn new_text_for_update(
    //     &self,
    //     old_text: String,
    //     update: impl FnOnce(&mut SettingsContent),
    // ) -> Result<String> {
    //     let edits = self.edits_for_update(&old_text, update)?;
    //     let mut new_text = old_text;
    //     for (range, replacement) in edits.into_iter() {
    //         new_text.replace_range(range, &replacement);
    //     }
    //     Ok(new_text)
    // }

    /// Updates the value of a setting in a JSON file, returning a list
    /// of edits to apply to the JSON file.
    // pub fn edits_for_update(
    //     &self,
    //     text: &str,
    //     update: impl FnOnce(&mut SettingsContent),
    // ) -> Result<Vec<(Range<usize>, String)>> {
    //     let old_content = if text.trim().is_empty() {
    //         UserSettingsContent::default()
    //     } else {
    //         let (old_content, parse_status) = UserSettingsContent::parse_json(text);
    //         if let ParseStatus::Failed { error } = &parse_status {
    //             log::error!("Failed to parse settings for update: {error}");
    //         }
    //         old_content
    //             .context("Settings file could not be parsed. Fix syntax errors before updating.")?
    //     };
    //     let mut new_content = old_content.clone();
    //     update(&mut new_content.content);

    //     let old_value = serde_json::to_value(&old_content).unwrap();
    //     let new_value = serde_json::to_value(new_content).unwrap();

    //     let mut key_path = Vec::new();
    //     let mut edits = Vec::new();
    //     let tab_size = infer_json_indent_size(&text);
    //     let mut text = text.to_string();
    //     update_value_in_json_text(
    //         &mut text,
    //         &mut key_path,
    //         tab_size,
    //         &old_value,
    //         &new_value,
    //         &mut edits,
    //     );
    //     Ok(edits)
    // }

    /// Mutates the default settings in place and recomputes all setting values.
    pub fn update_default_settings(
        &mut self,
        cx: &mut App,
        update: impl FnOnce(&mut SettingsContent),
    ) {
        let default_settings = Rc::make_mut(&mut self.default_settings);
        update(default_settings);
        // self.recompute_values(None, cx);
    }

    /// Sets the default settings via a JSON string.
    ///
    /// The string should contain a JSON object with a default value for every setting.
    pub fn set_default_settings(
        &mut self,
        default_settings_content: &str,
        cx: &mut App,
    ) -> anyhow::Result<()> {
        self.default_settings = Self::parse_default_settings(default_settings_content)?.into();
        // self.recompute_values(None, cx);
        Ok(())
    }

    /// Parses the default settings JSON and folds any `dev`/`nightly`/`preview`/`stable`
    /// release-channel overrides and `macos`/`linux`/`windows` platform overrides into
    /// the returned [`SettingsContent`].
    ///
    /// Unlike user settings, default settings are used directly as the base for all
    /// merges, so overrides must be resolved up front.
    fn parse_default_settings(default_settings: &str) -> anyhow::Result<SettingsContent> {
        let parsed: SettingsContent = parse_json_with_comments(default_settings)
            .context("Failed to parse default settings")?;
        // let mut merged = (*parsed.content).clone();
        // merged.merge_from_option(parsed.for_release_channel());
        // merged.merge_from_option(parsed.for_os());
        Ok(parsed)
    }
}

impl std::fmt::Debug for SettingsStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsStore")
            .field(
                "types",
                &self
                    .setting_values
                    .values()
                    .map(|value| value.setting_type_name())
                    .collect::<Vec<_>>(),
            )
            .field("default_settings", &self.default_settings)
            .field("user_settings", &self.user_settings)
            // .field("local_settings", &self.local_settings)
            .finish_non_exhaustive()
    }
}
