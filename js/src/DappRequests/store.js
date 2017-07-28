// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import { flatten } from 'lodash';
import { action, computed, observable } from 'mobx';
import store from 'store';

import { sha3 } from '@parity/api/util/sha3';

import VisibleStore from '../Dapps/dappsStore';
import filteredRequests from './filteredRequests';

const LS_PERMISSIONS = '_parity::dapps::methods';

let nextQueueId = 0;

export default class Store {
  @observable permissions = {};
  @observable requests = [];
  @observable tokens = {};

  constructor (provider) {
    this.provider = provider;
    this.permissions = store.get(LS_PERMISSIONS) || {};

    window.addEventListener('message', this.receiveMessage, false);
  }

  @computed get hasRequests () {
    return this.requests.length !== 0;
  }

  @computed get squashedRequests () {
    const duplicates = {};

    return this.requests.filter(({ request: { data: { method, token } } }) => {
      const section = this.getFilteredSectionName(method);
      const id = `${token}:${section}`;

      if (!duplicates[id]) {
        duplicates[id] = true;
        return true;
      }

      return false;
    });
  }

  @action createToken = (appId) => {
    const token = sha3(`${appId}:${Date.now()}`);

    this.tokens = Object.assign({}, this.tokens, {
      [token]: appId
    });

    return token;
  }

  @action removeRequest = (_queueId) => {
    this.requests = this.requests.filter(({ queueId }) => queueId !== _queueId);
  }

  @action queueRequest = (request) => {
    const appId = this.tokens[request.data.token];
    let queueId = ++nextQueueId;

    this.requests = this.requests.concat([{ appId, queueId, request }]);
  }

  @action addTokenPermission = (method, token) => {
    const id = `${method}:${this.tokens[token]}`;

    this.permissions = Object.assign({}, this.permissions, {
      [id]: true
    });
    this.savePermissions();
  }

  @action approveSingleRequest = ({ queueId, request: { data, source } }) => {
    this.removeRequest(queueId);
    if (data.api) {
      this.executePubsubCall(data, source);
    } else {
      this.executeMethodCall(data, source);
    }
  }

  @action approveRequest = (queueId, approveAll) => {
    const queued = this.findRequest(queueId);

    if (approveAll) {
      const { request: { data: { method, token, params } } } = queued;

      this.getFilteredSection(method || params[0]).methods.forEach((m) => {
        this.addTokenPermission(m, token);
        this.findMatchingRequests(m, token).forEach(this.approveSingleRequest);
      });
    } else {
      this.approveSingleRequest(queued);
    }
  }

  @action rejectRequest = (queueId) => {
    const { request } = this.findRequest(queueId);

    this.removeRequest(queueId);
    this.rejectMessage(request);
  }

  @action rejectMessage = (source, { id, from, method, token }) => {
    source.postMessage({
      error: `Method ${method} not allowed`,
      id,
      from: 'shell',
      result: null,
      to: from,
      token
    }, '*');
  }

  @action setPermissions = (_permissions) => {
    const permissions = {};

    Object.keys(_permissions).forEach((id) => {
      permissions[id] = !!_permissions[id];
    });

    this.permissions = Object.assign({}, this.permissions, permissions);
    this.savePermissions();

    return true;
  }

  hasValidToken = (method, appId, token) => {
    if (!token) {
      return method === 'shell_requestNewToken';
    }

    return this.tokens[token] === appId;
  }

  hasTokenPermission = (method, token) => {
    return this.hasAppPermission(method, this.tokens[token]);
  }

  hasAppPermission = (method, appId) => {
    return this.permissions[`${method}:${appId}`] || false;
  }

  savePermissions = () => {
    store.set(LS_PERMISSIONS, this.permissions);
  }

  findRequest (_queueId) {
    return this.requests.find(({ queueId }) => queueId === _queueId);
  }

  findMatchingRequests (_method, _token) {
    return this.requests.filter(({ request: { data: { method, token, params } } }) => (method === _method || (params && params[0] === _method)) && token === _token);
  }

  _methodCallbackPost = (id, from, source, token) => {
    return (error, result) => {
      source.postMessage({
        error: error
          ? error.message
          : null,
        id,
        from: 'shell',
        to: from,
        result,
        token
      }, '*');
    };
  }

  executePubsubCall = ({ api, id, from, token, params }, source) => {
    const callback = this._methodCallbackPost(id, from, source, token);

    // TODO: enable security pubsub
    this.provider.subscribe(api, callback, params).then((v, e) => {
      console.log('Error and result', v, e);
      this._methodCallbackPost(id, from, source, token)(null, v);
    });
  }

  executeMethodCall = ({ appId, id, from, method, params, token }, source) => {
    const visibleStore = VisibleStore.get();
    const callback = this._methodCallbackPost(id, from, source, token);

    switch (method) {
      case 'shell_getApps':
        const [displayAll] = params;

        return callback(null, displayAll
          ? visibleStore.allApps.slice()
          : visibleStore.visibleApps.slice()
        );

      case 'shell_getFilteredMethods':
        return callback(null, flatten(
          Object
            .keys(filteredRequests)
            .map((key) => filteredRequests[key].methods)
        ));

      case 'shell_getMethodPermissions':
        return callback(null, this.permissions);

      case 'shell_requestNewToken':
        return callback(null, this.createToken(from));

      case 'shell_setAppVisibility':
        const [appId, visibility] = params;

        return callback(null, visibility
          ? visibleStore.showApp(appId)
          : visibleStore.hideApp(appId)
        );

      case 'shell_setMethodPermissions':
        const [permissions] = params;

        return callback(null, this.setPermissions(permissions));

      default:
        return this.provider.send(method, params, callback);
    }
  }

  getFilteredSectionName = (method) => {
    return Object.keys(filteredRequests).find((key) => {
      return filteredRequests[key].methods.includes(method);
    });
  }

  getFilteredSection = (method) => {
    return filteredRequests[this.getFilteredSectionName(method)];
  }

  receiveMessage = ({ data, origin, source }) => {
    if (!data) {
      return;
    }

    const { from, method, to, token, params, api, subId, id } = data;

    if (to !== 'shell' || !from || from === 'shell') {
      return;
    }

    if (!this.hasValidToken(method, from, token)) {
      this.rejectMessage(source, data);
      return;
    }

    if ((method && this.getFilteredSection(method) && !this.hasTokenPermission(method, token)) ||
        (api && this.getFilteredSection(params[0]) && !this.hasTokenPermission(method, token))) {
      this.queueRequest({ data, origin, source });
      return;
    }

    if (api) {
      this.executePubsubCall(data, source);
    } else if (subId) {
      subId === '*'
        ? this.provider.unsubscribeAll().then(v => this._methodCallbackPost(id, from, source, token)(null, v))
        : this.provider.unsubscribe(subId).then(v => this._methodCallbackPost(id, from, source, token)(null, v));
    } else {
      this.executeMethodCall(data, source);
    }
  }

  static instance = null;

  static create (provider) {
    if (!Store.instance) {
      Store.instance = new Store(provider, {});
    }

    return Store.instance;
  }

  static get () {
    return Store.instance;
  }
}
