export declare const COMMAND: {
    CHECK_ACCESSIBILITY_PERMISSION: string;
    REQUEST_ACCESSIBILITY_PERMISSION: string;
    CHECK_FULL_DISK_ACCESS_PERMISSION: string;
    REQUEST_FULL_DISK_ACCESS_PERMISSION: string;
    CHECK_SCREEN_RECORDING_PERMISSION: string;
    REQUEST_SCREEN_RECORDING_PERMISSION: string;
    CHECK_MICROPHONE_PERMISSION: string;
    REQUEST_MICROPHONE_PERMISSION: string;
    CHECK_CAMERA_PERMISSION: string;
    REQUEST_CAMERA_PERMISSION: string;
    CHECK_INPUT_MONITORING_PERMISSION: string;
    REQUEST_INPUT_MONITORING_PERMISSION: string;
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
export declare const checkAccessibilityPermission: () => Promise<boolean>;
/**
 * Request accessibility permission.
 *
 * @example
 * import { requestAccessibilityPermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestAccessibilityPermission();
 */
export declare const requestAccessibilityPermission: () => Promise<unknown>;
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
export declare const checkFullDiskAccessPermission: () => Promise<boolean>;
/**
 * Request full disk access permission.
 *
 * @example
 * import { requestFullDiskAccessPermission } from "tauri-plugin-macos-permission-api";
 *
 * await requestFullDiskAccessPermission();
 */
export declare const requestFullDiskAccessPermission: () => Promise<unknown>;
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
export declare const checkScreenRecordingPermission: () => Promise<boolean>;
/**
 * Request screen recording permission.
 *
 * @example
 * import { requestScreenRecordingPermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestScreenRecordingPermission();
 */
export declare const requestScreenRecordingPermission: () => Promise<unknown>;
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
export declare const checkMicrophonePermission: () => Promise<boolean>;
/**
 * Request microphone permission.
 *
 * @example
 * import { requestMicrophonePermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestMicrophonePermission();
 */
export declare const requestMicrophonePermission: () => Promise<unknown>;
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
export declare const checkCameraPermission: () => Promise<boolean>;
/**
 * Request camera permission.
 *
 * @example
 * import { requestCameraPermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestCameraPermission();
 */
export declare const requestCameraPermission: () => Promise<unknown>;
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
export declare const checkInputMonitoringPermission: () => Promise<boolean>;
/**
 * Request input monitoring permission.
 *
 * @example
 * import { requestInputMonitoringPermission } from "tauri-plugin-macos-permissions-api";
 *
 * await requestInputMonitoringPermission();
 */
export declare const requestInputMonitoringPermission: () => Promise<unknown>;
