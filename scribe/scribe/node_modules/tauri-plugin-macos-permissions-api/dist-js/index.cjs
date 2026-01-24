'use strict';

var core = require('@tauri-apps/api/core');

const COMMAND = {
    CHECK_ACCESSIBILITY_PERMISSION: "plugin:macos-permissions|check_accessibility_permission",
    REQUEST_ACCESSIBILITY_PERMISSION: "plugin:macos-permissions|request_accessibility_permission",
    CHECK_FULL_DISK_ACCESS_PERMISSION: "plugin:macos-permissions|check_full_disk_access_permission",
    REQUEST_FULL_DISK_ACCESS_PERMISSION: "plugin:macos-permissions|request_full_disk_access_permission",
    CHECK_SCREEN_RECORDING_PERMISSION: "plugin:macos-permissions|check_screen_recording_permission",
    REQUEST_SCREEN_RECORDING_PERMISSION: "plugin:macos-permissions|request_screen_recording_permission",
    CHECK_MICROPHONE_PERMISSION: "plugin:macos-permissions|check_microphone_permission",
    REQUEST_MICROPHONE_PERMISSION: "plugin:macos-permissions|request_microphone_permission",
    CHECK_CAMERA_PERMISSION: "plugin:macos-permissions|check_camera_permission",
    REQUEST_CAMERA_PERMISSION: "plugin:macos-permissions|request_camera_permission",
    CHECK_INPUT_MONITORING_PERMISSION: "plugin:macos-permissions|check_input_monitoring_permission",
    REQUEST_INPUT_MONITORING_PERMISSION: "plugin:macos-permissions|request_input_monitoring_permission",
};
/**
 * Check accessibility permission.
 *
 * @returns `true` if accessibility permission are granted, `false` otherwise.
 *
 * @example
 * import { checkAccessibilityPermission } from "tauri-plugin-macos-permissions-api";
 *
 * const authorized = await checkAccessibilityPermission();
 * console.log(authorized); // false
 */
const checkAccessibilityPermission = () => {
    return core.invoke(COMMAND.CHECK_ACCESSIBILITY_PERMISSION);
};
/**
 * Request accessibility permission.
 *
 * @example
 * import { requestAccessibilityPermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestAccessibilityPermission();
 */
const requestAccessibilityPermission = () => {
    return core.invoke(COMMAND.REQUEST_ACCESSIBILITY_PERMISSION);
};
/**
 * Check full disk access permission.
 *
 * @returns `true` if full disk access permission are granted, `false` otherwise.
 *
 * @example
 * import { checkFullDiskAccessPermission } from "tauri-plugin-macos-permissions-api";
 *
 * const authorized = await checkFullDiskAccessPermission();
 * console.log(authorized); // false
 */
const checkFullDiskAccessPermission = () => {
    return core.invoke(COMMAND.CHECK_FULL_DISK_ACCESS_PERMISSION);
};
/**
 * Request full disk access permission.
 *
 * @example
 * import { requestFullDiskAccessPermission } from "tauri-plugin-macos-permission-api";
 *
 * await requestFullDiskAccessPermission();
 */
const requestFullDiskAccessPermission = () => {
    return core.invoke(COMMAND.REQUEST_FULL_DISK_ACCESS_PERMISSION);
};
/**
 * Check screen recording permission.
 *
 * @returns `true` if screen recording permission are granted, `false` otherwise.
 *
 * @example
 * import { checkScreenRecordingPermission } from "tauri-plugin-macos-permissions-api";
 *
 * const authorized = await checkScreenRecordingPermission();
 * console.log(authorized); // false
 */
const checkScreenRecordingPermission = () => {
    return core.invoke(COMMAND.CHECK_SCREEN_RECORDING_PERMISSION);
};
/**
 * Request screen recording permission.
 *
 * @example
 * import { requestScreenRecordingPermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestScreenRecordingPermission();
 */
const requestScreenRecordingPermission = () => {
    return core.invoke(COMMAND.REQUEST_SCREEN_RECORDING_PERMISSION);
};
/**
 * Check microphone permission.
 *
 * @returns `true` if microphone permission are granted, `false` otherwise.
 *
 * @example
 * import { checkMicrophonePermission } from "tauri-plugin-macos-permissions-api";
 *
 * const authorized = await checkMicrophonePermission();
 * console.log(authorized); // false
 */
const checkMicrophonePermission = () => {
    return core.invoke(COMMAND.CHECK_MICROPHONE_PERMISSION);
};
/**
 * Request microphone permission.
 *
 * @example
 * import { requestMicrophonePermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestMicrophonePermission();
 */
const requestMicrophonePermission = () => {
    return core.invoke(COMMAND.REQUEST_MICROPHONE_PERMISSION);
};
/**
 * Check camera permission.
 *
 * @returns `true` if camera permission are granted, `false` otherwise.
 *
 * @example
 * import { checkCameraPermission } from "tauri-plugin-macos-permissions-api";
 *
 * const authorized = await checkCameraPermission();
 * console.log(authorized); // false
 */
const checkCameraPermission = () => {
    return core.invoke(COMMAND.CHECK_CAMERA_PERMISSION);
};
/**
 * Request camera permission.
 *
 * @example
 * import { requestCameraPermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestCameraPermission();
 */
const requestCameraPermission = () => {
    return core.invoke(COMMAND.REQUEST_CAMERA_PERMISSION);
};
/**
 * Check input monitoring permission.
 *
 * @returns `true` if input monitoring permission are granted, `false` otherwise.
 *
 * @example
 * import { checkInputMonitoringPermission } from "tauri-plugin-macos-permissions-api";
 *
 * const authorized = await checkInputMonitoringPermission();
 * console.log(authorized); // false
 */
const checkInputMonitoringPermission = () => {
    return core.invoke(COMMAND.CHECK_INPUT_MONITORING_PERMISSION);
};
/**
 * Request input monitoring permission.
 *
 * @example
 * import { requestInputMonitoringPermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestInputMonitoringPermission();
 */
const requestInputMonitoringPermission = () => {
    return core.invoke(COMMAND.REQUEST_INPUT_MONITORING_PERMISSION);
};

exports.COMMAND = COMMAND;
exports.checkAccessibilityPermission = checkAccessibilityPermission;
exports.checkCameraPermission = checkCameraPermission;
exports.checkFullDiskAccessPermission = checkFullDiskAccessPermission;
exports.checkInputMonitoringPermission = checkInputMonitoringPermission;
exports.checkMicrophonePermission = checkMicrophonePermission;
exports.checkScreenRecordingPermission = checkScreenRecordingPermission;
exports.requestAccessibilityPermission = requestAccessibilityPermission;
exports.requestCameraPermission = requestCameraPermission;
exports.requestFullDiskAccessPermission = requestFullDiskAccessPermission;
exports.requestInputMonitoringPermission = requestInputMonitoringPermission;
exports.requestMicrophonePermission = requestMicrophonePermission;
exports.requestScreenRecordingPermission = requestScreenRecordingPermission;
