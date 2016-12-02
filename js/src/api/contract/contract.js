// Copyright 2015, 2016 Ethcore (UK) Ltd.
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

import Abi from '../../abi';

let nextSubscriptionId = 0;

export default class Contract {
  constructor (api, abi) {
    if (!api) {
      throw new Error('API instance needs to be provided to Contract');
    }

    if (!abi) {
      throw new Error('ABI needs to be provided to Contract instance');
    }

    this._api = api;
    this._abi = new Abi(abi);

    this._subscriptions = {};
    this._constructors = this._abi.constructors.map(this._bindFunction);
    this._functions = this._abi.functions.map(this._bindFunction);
    this._events = this._abi.events.map(this._bindEvent);

    this._instance = {};

    this._events.forEach((evt) => {
      this._instance[evt.name] = evt;
      this._instance[evt.signature] = evt;
    });

    this._functions.forEach((fn) => {
      this._instance[fn.name] = fn;
      this._instance[fn.signature] = fn;
    });

    this._subscribedToPendings = false;
    this._pendingsSubscriptionId = null;

    this._subscribedToBlock = false;
    this._blockSubscriptionId = null;
  }

  get address () {
    return this._address;
  }

  get constructors () {
    return this._constructors;
  }

  get events () {
    return this._events;
  }

  get functions () {
    return this._functions;
  }

  get instance () {
    this._instance.address = this._address;
    return this._instance;
  }

  get api () {
    return this._api;
  }

  get abi () {
    return this._abi;
  }

  at (address) {
    this._address = address;
    return this;
  }

  deploy (options, values, statecb) {
    let gas;

    const setState = (state) => {
      if (!statecb) {
        return;
      }

      return statecb(null, state);
    };

    setState({ state: 'estimateGas' });

    return this._api.eth
      .estimateGas(this._encodeOptions(this.constructors[0], options, values))
      .then((_gas) => {
        gas = _gas.mul(1.2);
        options.gas = gas.toFixed(0);

        setState({ state: 'postTransaction', gas });
        return this._api.parity.postTransaction(this._encodeOptions(this.constructors[0], options, values));
      })
      .then((requestId) => {
        setState({ state: 'checkRequest', requestId });
        return this._pollCheckRequest(requestId);
      })
      .then((txhash) => {
        setState({ state: 'getTransactionReceipt', txhash });
        return this._pollTransactionReceipt(txhash, gas);
      })
      .then((receipt) => {
        if (receipt.gasUsed.eq(gas)) {
          throw new Error(`Contract not deployed, gasUsed == ${gas.toFixed(0)}`);
        }

        setState({ state: 'hasReceipt', receipt });
        this._address = receipt.contractAddress;
        return this._address;
      })
      .then((address) => {
        setState({ state: 'getCode' });
        return this._api.eth.getCode(this._address);
      })
      .then((code) => {
        if (code === '0x') {
          throw new Error('Contract not deployed, getCode returned 0x');
        }

        setState({ state: 'completed' });
        return this._address;
      });
  }

  parseEventLogs (logs) {
    return logs
      .map((log) => {
        const signature = log.topics[0].substr(2);
        const event = this.events.find((evt) => evt.signature === signature);

        if (!event) {
          console.warn(`Unable to find event matching signature ${signature}`);
          return null;
        }

        const decoded = event.decodeLog(log.topics, log.data);

        log.params = {};
        log.event = event.name;

        decoded.params.forEach((param) => {
          const { type, value } = param.token;

          log.params[param.name] = { type, value };
        });

        return log;
      })
      .filter((log) => log);
  }

  parseTransactionEvents (receipt) {
    receipt.logs = this.parseEventLogs(receipt.logs);

    return receipt;
  }

  _pollCheckRequest = (requestId) => {
    return this._api.pollMethod('parity_checkRequest', requestId);
  }

  _pollTransactionReceipt = (txhash, gas) => {
    return this.api.pollMethod('eth_getTransactionReceipt', txhash, (receipt) => {
      if (!receipt || !receipt.blockNumber || receipt.blockNumber.eq(0)) {
        return false;
      }

      return true;
    });
  }

  _encodeOptions (func, options, values) {
    const tokens = func ? this._abi.encodeTokens(func.inputParamTypes(), values) : null;
    const call = tokens ? func.encodeCall(tokens) : null;

    if (options.data && options.data.substr(0, 2) === '0x') {
      options.data = options.data.substr(2);
    }
    options.data = `0x${options.data || ''}${call || ''}`;

    return options;
  }

  _addOptionsTo (options = {}) {
    return Object.assign({
      to: this._address
    }, options);
  }

  _bindFunction = (func) => {
    func.call = (options, values = []) => {
      return this._api.eth
        .call(this._encodeOptions(func, this._addOptionsTo(options), values))
        .then((encoded) => func.decodeOutput(encoded))
        .then((tokens) => tokens.map((token) => token.value))
        .then((returns) => returns.length === 1 ? returns[0] : returns);
    };

    if (!func.constant) {
      func.postTransaction = (options, values = []) => {
        return this._api.parity
          .postTransaction(this._encodeOptions(func, this._addOptionsTo(options), values));
      };

      func.estimateGas = (options, values = []) => {
        return this._api.eth
          .estimateGas(this._encodeOptions(func, this._addOptionsTo(options), values));
      };
    }

    return func;
  }

  _bindEvent = (event) => {
    event.subscribe = (options = {}, callback) => {
      return this._subscribe(event, options, callback);
    };

    event.unsubscribe = (subscriptionId) => {
      return this.unsubscribe(subscriptionId);
    };

    return event;
  }

  _findEvent (eventName = null) {
    const event = eventName
      ? this._events.find((evt) => evt.name === eventName)
      : null;

    if (eventName && !event) {
      const events = this._events.map((evt) => evt.name).join(', ');
      throw new Error(`${eventName} is not a valid eventName, subscribe using one of ${events} (or null to include all)`);
    }

    return event;
  }

  _createEthFilter (event = null, _options) {
    const optionTopics = _options.topics || [];
    const signature = event && event.signature || null;

    // If event provided, remove the potential event signature
    // as the first element of the topics
    const topics = signature
      ? [ signature ].concat(optionTopics.filter((t, idx) => idx > 0 || t !== signature))
      : optionTopics;

    const options = Object.assign({}, _options, {
      address: this._address,
      topics
    });

    return this._api.eth.newFilter(options);
  }

  subscribe (eventName = null, options = {}, callback) {
    try {
      const event = this._findEvent(eventName);
      return this._subscribe(event, options, callback);
    } catch (e) {
      return Promise.reject(e);
    }
  }

  _subscribe (event = null, _options, callback) {
    const subscriptionId = nextSubscriptionId++;
    const { skipInitFetch } = _options;
    delete _options['skipInitFetch'];

    return this
      ._createEthFilter(event, _options)
      .then((filterId) => {
        this._subscriptions[subscriptionId] = {
          options: _options,
          callback,
          filterId
        };

        if (skipInitFetch) {
          this._subscribeToChanges();
          return subscriptionId;
        }

        return this._api.eth
          .getFilterLogs(filterId)
          .then((logs) => {
            callback(null, this.parseEventLogs(logs));

            this._subscribeToChanges();
            return subscriptionId;
          });
      });
  }

  unsubscribe (subscriptionId) {
    return this._api.eth
      .uninstallFilter(this._subscriptions[subscriptionId].filterId)
      .catch((error) => {
        console.error('unsubscribe', error);
      })
      .then(() => {
        delete this._subscriptions[subscriptionId];
        this._unsubscribeFromChanges();
      });
  }

  _subscribeToChanges = () => {
    const subscriptions = Object.values(this._subscriptions);

    const pendingSubscriptions = subscriptions
      .filter((s) => s.options.toBlock && s.options.toBlock === 'pending');

    const otherSubscriptions = subscriptions
      .filter((s) => !(s.options.toBlock && s.options.toBlock === 'pending'));

    if (pendingSubscriptions.length > 0 && !this._subscribedToPendings) {
      this._subscribedToPendings = true;
      this._subscribeToPendings();
    }

    if (otherSubscriptions.length > 0 && !this._subscribedToBlock) {
      this._subscribedToBlock = true;
      this._subscribeToBlock();
    }
  }

  _unsubscribeFromChanges = () => {
    const subscriptions = Object.values(this._subscriptions);

    const pendingSubscriptions = subscriptions
      .filter((s) => s.options.toBlock && s.options.toBlock === 'pending');

    const otherSubscriptions = subscriptions
      .filter((s) => !(s.options.toBlock && s.options.toBlock === 'pending'));

    if (pendingSubscriptions.length === 0 && this._subscribedToPendings) {
      this._subscribedToPendings = false;
      clearTimeout(this._pendingsSubscriptionId);
    }

    if (otherSubscriptions.length === 0 && this._subscribedToBlock) {
      this._subscribedToBlock = false;
      this._api.unsubscribe(this._blockSubscriptionId);
    }
  }

  _subscribeToBlock = () => {
    this._api
      .subscribe('eth_blockNumber', (error) => {
        if (error) {
          console.error('::_subscribeToBlock', error, error && error.stack);
        }

        const subscriptions = Object.values(this._subscriptions)
          .filter((s) => !(s.options.toBlock && s.options.toBlock === 'pending'));

        this._sendSubscriptionChanges(subscriptions);
      })
      .then((blockSubId) => {
        this._blockSubscriptionId = blockSubId;
      })
      .catch((e) => {
        console.error('::_subscribeToBlock', e, e && e.stack);
      });
  }

  _subscribeToPendings = () => {
    const subscriptions = Object.values(this._subscriptions)
      .filter((s) => s.options.toBlock && s.options.toBlock === 'pending');

    const timeout = () => setTimeout(() => this._subscribeToPendings(), 1000);

    this._sendSubscriptionChanges(subscriptions)
      .then(() => {
        this._pendingsSubscriptionId = timeout();
      });
  }

  _sendSubscriptionChanges = (subscriptions) => {
    return Promise
      .all(
        subscriptions.map((subscription) => {
          return this._api.eth.getFilterChanges(subscription.filterId);
        })
      )
      .then((logsArray) => {
        logsArray.forEach((logs, idx) => {
          if (!logs || !logs.length) {
            return;
          }

          try {
            subscriptions[idx].callback(null, this.parseEventLogs(logs));
          } catch (error) {
            console.error('_sendSubscriptionChanges', error);
          }
        });
      })
      .catch((error) => {
        console.error('_sendSubscriptionChanges', error);
      });
  }
}
