#!/bin/bash
# 构建结果通知：成功失败都执行（if: always()），结果写进 Step Summary + 日志
set -u
STATUS="${BUILD_STATUS:-unknown}"
SHA="${HEAD_SHA:-${GITHUB_SHA:-unknown}}"
RUN_ID="${RUN_ID:-${GITHUB_RUN_ID:-unknown}}"
APK="ech-demo-pure/app/build/outputs/apk/debug/app-debug.apk"

{
  echo "## 构建结果 ${STATUS}"
  echo ""
  echo "- 提交 SHA: ${SHA}"
  echo "- 运行 ID: ${RUN_ID}"
  echo "- 结论: ${STATUS}"
  if [ "${STATUS}" = "success" ]; then
    if [ -f "${APK}" ]; then
      echo "- APK: app-debug.apk ($(stat -c%s "${APK}") 字节)"
    else
      echo "- APK: 未找到（构建成功但产物缺失）"
    fi
  fi
} >> "${GITHUB_STEP_SUMMARY:-/dev/null}"

if [ "${STATUS}" = "success" ]; then
  if [ -f "${APK}" ]; then
    echo "BUILD_OK sha=${SHA} run=${RUN_ID} size=$(stat -c%s "${APK}")"
  else
    echo "BUILD_OK_BUT_NO_APK sha=${SHA} run=${RUN_ID}"
    exit 1
  fi
else
  echo "BUILD_FAIL sha=${SHA} run=${RUN_ID}"
  exit 1
fi
