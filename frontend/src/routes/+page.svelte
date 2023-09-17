<script lang="ts">
	const hostname = 'https://bipper.maxkienitz.com/api';

	let words = Array(24).fill('');
	let inputText = '';
	let fileInput: HTMLInputElement;
	let selectedFilename: string = 'No file selected';
	let selectedFile: File;
	let disableUploadButton = true;

	function clearInput() {
		inputText = '';
	}
	function downloadBip() {
		inputText = '';
	}
	function deleteBip() {
		inputText = '';
	}

	async function uploadBip() {
		const response = await fetch(`${hostname}/store/${selectedFilename}`, {
			method: 'POST',
			body: selectedFile
		});
		response.text().then((text) => {
			console.log(text);
			inputText = text;
		});
	}

	function onFileSelected() {
		selectedFile = fileInput.files![0];
		selectedFilename = selectedFile.name;
		disableUploadButton = false;
	}

	$: {
		let splitWords = inputText.trim().split(/\s+/).slice(0, 24);
		words = [...splitWords, ...Array(24 - splitWords.length).fill('')];
	}
</script>

<div class="flex flex-col space-y-2">
	<div class="flex flex-col space-y-2">
		<div class="flex flex-col justify-center h-32 items-center rounded-xl border border-dashed">
			<p class="p-4">Drag a file here to upload!</p>
			<div class="flex items-center p-2 justify-center">{selectedFilename}</div>
		</div>
		<input style="display:none" bind:this={fileInput} on:change={onFileSelected} type="file" />
		<div class="flex flex-row space-x-2">
			<button
				class="p-2 truncate border flex-grow"
				on:click={() => {
					fileInput.click();
				}}>Click to select File</button
			>
			<button class="p-2 border truncate" on:click={uploadBip} disabled={disableUploadButton}
				>Upload</button
			>
		</div>
	</div>
	<div class="flex flex-row border divide-x">
		<input
			bind:value={inputText}
			class="flex-grow bg-transparent p-2 text-center truncate"
			placeholder="Enter your BIP39 passphrase"
		/>
		<button on:click={clearInput} class="p-2">Clear</button>
	</div>
	<div class="flex flex-wrap border">
		{#each words as word}
			<div class="flex basis-1/6 grow items-center justify-center text-center truncate">
				{word || '*'}
			</div>
		{/each}
	</div>
	<div class="flex flex-row space-x-2">
		<button on:click={downloadBip} class="p-2 border basis-1/2">Download</button>
		<button on:click={deleteBip} class="p-2 border basis-1/2">Delete</button>
	</div>
</div>

<style lang="postcss">
	:global(button) {
		background-color: theme(colors.gray.600);
	}
</style>
