# Check native Windows integration

Use the running Jev inspector and `tests/fixtures/native-check.jev.json`. This fixture contains two synthetic exchanges and no credentials. Opening it freezes live updates in the inspector until you select **Return to live**.

## Check copying and file dialogs

1. Press **Ctrl+O** and open `tests/fixtures/native-check.jev.json` from this repository. The header says **Saved session**, and two requests appear. Exchange **#002** has a summary only. Exchange **#001** has JSON bodies and a star.
2. Select **#002**, click **Copy request ID** beside the endpoint, and paste into Notepad. Expect exactly `native-qa-summary-only`.
3. Select **#001**, open **Response**, and choose **Tree**. Select the `message` property and click **Copy value**. Paste into Notepad. Expect `"native clipboard check"`, including the JSON quotation marks. The inspector also identifies the copied property in a notification.
4. Select **#002** and star it. Press **Ctrl+E**, choose **Save session**, and cancel the Windows Save dialog. Both rows and their stars remain. Press **Ctrl+O** and cancel the Windows Open dialog; the view remains unchanged.
5. Export a session again and save it as `native-check-saved.jev.json` in a location you choose. Open that saved file with **Ctrl+O**. Expect two starred exchanges, #001's JSON bodies, and #002's summary-only explanation.

## Check native keyboard and accessibility behavior

1. Use **Tab** to reach Traffic, its filters, the request table, the divider, and the detail tabs. Focus remains visible. Arrow keys move table selection. On the divider, **Left** and **Right** resize the list.
2. In the detail tab group, **Left/Right** move focus and **Enter** selects the focused tab. In the JSON tree, arrow keys navigate and expand or collapse properties.
3. Open Help with **F1**. Tab stays inside the dialog. With Help content focused, **End** reaches the last paragraph. **Escape** closes Help and returns focus to the previous control.
4. With your Windows screen reader enabled, verify that it announces tab names and selected state, request selection, tree expansion state, and the Help dialog. Record the screen reader's name/version and any missing or misleading announcement.

Report each group as passed, failed, or not tested: clipboard; save/open/cancel; keyboard/focus; screen-reader announcements. For a failure, include the step, expected result, and observed result. Unperformed checks remain unverified.

These checks establish Windows behavior only. Run equivalent checks on macOS before claiming macOS integration is verified.
