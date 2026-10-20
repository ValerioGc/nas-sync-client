HTMLDialogElement.prototype.showModal = function () {
  this.open = true;
  this.querySelector<HTMLElement>('[autofocus]')?.focus();
};

HTMLDialogElement.prototype.close = function () {
  this.open = false;
};
